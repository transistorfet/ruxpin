
use core::ptr::NonNull;

use alloc::sync::Arc;

use ruxpin_types::{DeviceID, FileAccess, UserID, GroupID};

use ruxpin_kernel::block;
use ruxpin_kernel::misc::memory;
use ruxpin_kernel::{info, debug, trace};
use ruxpin_kernel::sync::Spinlock;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::misc::byteorder::{leu16, leu32};
use ruxpin_kernel::fs::types::{Vnode, FileAttributes};

use super::Ext2InodeNum;
use super::Ext2BlockNumber;
use super::mount::Ext2Mount;


pub(super) const EXT2_INODE_DIRECT_BLOCKS: usize           = 12;
pub(super) const EXT2_INODE_INDIRECT_BLOCKS: usize         = EXT2_INODE_DIRECT_BLOCKS + 1;
pub(super) const EXT2_INODE_DOUBLE_INDIRECT_BLOCKS: usize  = EXT2_INODE_INDIRECT_BLOCKS + 1;
pub(super) const EXT2_INODE_TRIPLE_INDIRECT_BLOCKS: usize  = EXT2_INODE_DOUBLE_INDIRECT_BLOCKS + 1;
pub(super) const EXT2_INODE_BLOCK_ENTRIES: usize           = EXT2_INODE_TRIPLE_INDIRECT_BLOCKS;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Ext2InodeOnDisk {
    mode: leu16,
    uid: leu16,
    size: leu32,
    atime: leu32,
    ctime: leu32,
    mtime: leu32,
    dtime: leu32,
    gid: leu16,
    num_links: leu16,
    num_blocks: leu32,
    flags: leu32,
    _reserved1: leu32,
    blocks: [leu32; EXT2_INODE_BLOCK_ENTRIES],
    file_generation: leu32,
    file_acl: leu32,
    dir_acl: leu32,
    fragment_addr: leu32,
    fragment_num: u8,
    fragment_size: u8,
    _pad1: leu16,
    uid_high: leu16,
    gid_high: leu16,
    _reserved2: leu32,
}


pub(super) struct Ext2Vnode {
    pub attrs: FileAttributes,
    pub mount_ptr: NonNull<Ext2Mount>,
    pub mounted_vnode: Option<Vnode>,
    pub blocks: [Ext2BlockNumber; EXT2_INODE_BLOCK_ENTRIES],
    pub dirty: bool,
    pub freed: bool,
}

unsafe impl Send for Ext2Vnode {}
unsafe impl Sync for Ext2Vnode {}


fn get_mount(mount_ptr: NonNull<Ext2Mount>) -> &'static mut Ext2Mount {
    unsafe {
        &mut *mount_ptr.as_ptr()
    }
}

impl Ext2Vnode {
    pub fn get_mount(&self) -> &mut Ext2Mount {
        get_mount(self.mount_ptr)
    }

    pub fn get_block_size(&self) -> usize {
        self.get_mount().superblock.get_block_size()
    }

    pub fn get_device_id(&self) -> DeviceID {
        self.get_mount().device_id
    }

    pub fn get_inode(&mut self, inode_num: Ext2InodeNum) -> Result<Vnode, KernelError> {
        self.get_mount().get_inode(inode_num)
    }

    pub fn writeback(&mut self) -> Result<(), KernelError> {
        if self.dirty && !self.freed {
            self.get_mount().store_inode(&self, self.attrs.inode)?;
        }
        Ok(())
    }

    pub fn free_inode(&mut self, inode_num: Ext2InodeNum) -> Result<(), KernelError> {
        info!("ext2: freeing inode {}", inode_num);
        self.get_mount().superblock.free_inode(inode_num)?;
        self.freed = true;
        Ok(())
    }
}

impl Ext2Vnode {
    fn new(mount_ptr: NonNull<Ext2Mount>, access: FileAccess, uid: UserID, gid: GroupID) -> Self {
        Self {
            attrs: FileAttributes::new(access, uid, gid),
            mount_ptr,
            mounted_vnode: None,
            blocks: [0; EXT2_INODE_BLOCK_ENTRIES],
            dirty: false,
            freed: false,
        }
    }

    fn new_default(mount_ptr: NonNull<Ext2Mount>) -> Self {
        Self {
            attrs: Default::default(),
            mount_ptr,
            mounted_vnode: None,
            blocks: [0; EXT2_INODE_BLOCK_ENTRIES],
            dirty: false,
            freed: false,
        }
    }
}


impl Ext2Mount {
    fn as_ptr(&mut self) -> NonNull<Self> {
        NonNull::new(self as *mut Self).unwrap()
    }

    pub(super) fn alloc_inode(&mut self, start_from: Ext2InodeNum, access: FileAccess, uid: UserID, gid: GroupID) -> Result<(Ext2InodeNum, Vnode), KernelError> {
        let mount_ptr = self.as_ptr();
        let inode_num = self.superblock.alloc_inode(start_from)?;
        let mut vnode = Ext2Vnode::new(mount_ptr, access, uid, gid);
        vnode.attrs.inode = inode_num;
        self.store_inode(&vnode, inode_num)?;
        info!("ext2: allocating inode {}", inode_num);

        // Insert the node into the cache
        let arc_vnode = self.vnode_cache.insert(inode_num, || {
            Ok(Arc::new(Spinlock::new(vnode)))
        }, |_, vnode| {
            vnode.lock().commit()
        })?;

        Ok((inode_num, (*arc_vnode).clone()))
    }

    pub(super) fn get_inode(&mut self, inode_num: Ext2InodeNum) -> Result<Vnode, KernelError> {
        let mount_ptr = self.as_ptr();
        let vnode = self.vnode_cache.get(inode_num, || {
            let mut vnode = Ext2Vnode::new_default(mount_ptr);
            get_mount(mount_ptr).load_inode(&mut vnode, inode_num)?;
            Ok(Arc::new(Spinlock::new(vnode)))
        }, |_, vnode| {
            vnode.lock().commit()
        })?;

        Ok((*vnode).clone())
    }

    pub(super) fn store_inodes(&mut self) -> Result<(), KernelError> {
        self.vnode_cache.commit(|_, vnode| {
            vnode.lock().commit()
        })
    }

    fn load_inode(&mut self, vnode: &mut Ext2Vnode, inode_num: Ext2InodeNum) -> Result<(), KernelError> {
        let (block_num, byte_offset) = self.superblock.get_inode_entry_location(inode_num)?;
        self.superblock.check_inode_is_allocated(inode_num)?;

        let buf = block::get_buf(self.device_id, block_num)?;
        let locked_buf = &*buf.lock();
        let data: &Ext2InodeOnDisk = unsafe {
            memory::cast_to_ref(&locked_buf[byte_offset..])
        };

        for i in 0..vnode.blocks.len() {
            vnode.blocks[i] = data.blocks[i].into();
        }

        vnode.attrs.inode = inode_num;
        vnode.attrs.access = u16::from(data.mode).into();
        vnode.attrs.nlinks = data.num_links.into();
        vnode.attrs.uid = data.uid.into();
        vnode.attrs.gid = data.gid.into();
        //vnode.attrs.rdev = None;
        vnode.attrs.size = u32::from(data.size) as usize;
        vnode.attrs.atime = u32::from(data.atime).into();
        vnode.attrs.mtime = u32::from(data.mtime).into();
        vnode.attrs.ctime = u32::from(data.ctime).into();

        trace!("loading inode {}: {:#?} {:#?}", inode_num, (*data), vnode.attrs);
        Ok(())
    }

    fn store_inode(&mut self, vnode: &Ext2Vnode, inode_num: Ext2InodeNum) -> Result<(), KernelError> {
        let (block_num, byte_offset) = self.superblock.get_inode_entry_location(inode_num)?;

        let buf = block::get_buf(self.device_id, block_num)?;
        let locked_buf = &mut *buf.lock_mut();
        let data: &mut Ext2InodeOnDisk = unsafe {
            memory::cast_to_ref_mut(&mut locked_buf[byte_offset..])
        };

        for i in 0..vnode.blocks.len() {
            data.blocks[i] = vnode.blocks[i].into();
        }

        data.mode = u16::from(vnode.attrs.access).into();
        data.num_links = vnode.attrs.nlinks.into();
        data.uid = vnode.attrs.uid.into();
        data.gid = vnode.attrs.gid.into();
        //data.rdev = None;
        data.size = (vnode.attrs.size as u32).into();
        data.atime = u32::from(vnode.attrs.atime).into();
        data.mtime = u32::from(vnode.attrs.mtime).into();
        data.ctime = u32::from(vnode.attrs.ctime).into();

        debug!("storing inode {}", inode_num);
        trace!("inode as written {}: {:#?}", inode_num, (*data));
        Ok(())
    }
}

