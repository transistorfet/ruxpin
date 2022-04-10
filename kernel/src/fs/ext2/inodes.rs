
use core::ptr::NonNull;

use alloc::sync::Arc;

use ruxpin_api::types::DeviceID;

use crate::block;
use crate::block::BlockNum;
use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::misc::byteorder::{leu16, leu32};
use crate::fs::types::{Vnode, FileAttributes};

use super::mount::Ext2Mount;


pub(super) type Ext2InodeNum = u32;
pub(super) type Ext2BlockNumber = u32;

const EXT2_INODE_DIRECT_BLOCKS: usize           = 12;
const EXT2_INODE_INDIRECT_BLOCKS: usize         = EXT2_INODE_DIRECT_BLOCKS + 1;
const EXT2_INODE_DOUBLE_INDIRECT_BLOCKS: usize  = EXT2_INODE_INDIRECT_BLOCKS + 1;
const EXT2_INODE_TRIPLE_INDIRECT_BLOCKS: usize  = EXT2_INODE_DOUBLE_INDIRECT_BLOCKS + 1;
const EXT2_INODE_BLOCK_ENTRIES: usize           = EXT2_INODE_TRIPLE_INDIRECT_BLOCKS;

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

impl Into<FileAttributes> for Ext2InodeOnDisk {
    fn into(self) -> FileAttributes {
        FileAttributes {
            access: u16::from(self.mode).into(),
            nlinks: self.num_links.into(),
            uid: self.uid.into(),
            gid: self.gid.into(),
            rdev: None,
            inode: 0,
            size: u32::from(self.size) as usize,

            atime: u32::from(self.atime).into(),
            mtime: u32::from(self.mtime).into(),
            ctime: u32::from(self.ctime).into(),
        }
    }
}

pub(super) struct Ext2Vnode {
    pub attrs: FileAttributes,
    pub mount_ptr: NonNull<Ext2Mount>,
    pub mounted_vnode: Option<Vnode>,
    pub blocks: [Ext2BlockNumber; EXT2_INODE_BLOCK_ENTRIES],
}

unsafe impl Send for Ext2Vnode {}
unsafe impl Sync for Ext2Vnode {}


impl Ext2Vnode {
    pub fn get_block_size(&self) -> usize {
        get_mount(self.mount_ptr).superblock.get_block_size()
    }

    pub fn get_device_id(&self) -> DeviceID {
        get_mount(self.mount_ptr).device_id
    }

    pub fn get_file_block_num(&self, linear_block_num: usize) -> Result<BlockNum, KernelError> {
        if linear_block_num < EXT2_INODE_DIRECT_BLOCKS {
            Ok(self.blocks[linear_block_num])
        } else {
            //lookup_indirect_block(self.get_device_id(), self.blocks[EXT2_INODE_INDIRECT_BLOCKS - 1], linear_block_num - EXT2_INODE_DIRECT_BLOCKS)
            //fn lookup_indirect_block(device_id: DeviceID, block_table: BlockNum, offset: usize) -> Result<BlockNum, KernelError> {
                //let buf = block::get_buf(device_id, block_table)?;
            //}
            panic!("Not implemented");
        }
    }

    pub fn get_inode(&mut self, inode_num: Ext2InodeNum) -> Result<Vnode, KernelError> {
        get_mount(self.mount_ptr).get_inode(inode_num)
    }
}

fn get_mount(mount_ptr: NonNull<Ext2Mount>) -> &'static mut Ext2Mount {
    unsafe {
        &mut *mount_ptr.as_ptr()
    }
}

impl Ext2Vnode {
    fn new(mount_ptr: NonNull<Ext2Mount>) -> Self {
        Self {
            attrs: Default::default(),
            mount_ptr,
            mounted_vnode: None,
            blocks: [0; EXT2_INODE_BLOCK_ENTRIES],
        }
    }
}


impl Ext2Mount {
    pub fn as_ptr(&mut self) -> NonNull<Self> {
        NonNull::new(self as *mut Self).unwrap()
    }

    pub fn get_inode(&mut self, inode_num: Ext2InodeNum) -> Result<Vnode, KernelError> {
        let mount_ptr = self.as_ptr();
        let vnode = self.vnode_cache.get(inode_num, || {
            let mut vnode = Ext2Vnode::new(mount_ptr);
            get_mount(mount_ptr).load_inode(&mut vnode, inode_num)?;
            Ok(Arc::new(Spinlock::new(vnode)))
        })?;

        Ok((*vnode).clone())
    }

    fn load_inode(&mut self, vnode: &mut Ext2Vnode, inode_num: Ext2InodeNum) -> Result<(), KernelError> {
        let (block_num, byte_offset) = self.superblock.get_inode_entry_location(inode_num)?;
        self.check_inode_allocated(inode_num)?;
        let buf = block::get_buf(self.device_id, block_num)?;

        let data = unsafe {
            &*((buf.block.lock()).as_ptr().add(byte_offset) as *mut Ext2InodeOnDisk)
        };

        for i in 0..vnode.blocks.len() {
            vnode.blocks[i] = data.blocks[i].into();
        }

        vnode.attrs = (*data).into();
        vnode.attrs.inode = inode_num;

        //crate::printkln!("loading inode {}: {:#?} {:#?}", inode_num, (*data), vnode.attrs);
        Ok(())
    }

    pub(super) fn check_inode_allocated(&self, inode_num: Ext2InodeNum) -> Result<(), KernelError> {
        let (bitmap, index) = self.superblock.get_inode_bitmap_location(inode_num)?;
        let buf = block::get_buf(self.device_id, bitmap)?;
        let data = buf.block.lock();

        if data[index / 8] & (1 << (index % 8)) != 0 {
            Ok(())
        } else {
            Err(KernelError::InvalidInode)
        }
    }
}

