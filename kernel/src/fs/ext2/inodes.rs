
use core::slice;
use core::ptr::NonNull;

use alloc::sync::Arc;

use crate::block;
use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::fs::types::{Vnode, FileAttributes};

use super::Ext2Mount;


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
    mode: u16,
    uid: u16,
    size: u32,
    atime: u32,
    ctime: u32,
    mtime: u32,
    dtime: u32,
    gid: u16,
    num_links: u16,
    num_blocks: u32,
    flags: u32,
    _reserved1: u32,
    blocks: [u32; EXT2_INODE_BLOCK_ENTRIES],
    file_generation: u32,
    file_acl: u32,
    dir_acl: u32,
    fragment_addr: u32,
    fragment_num: u8,
    fragment_size: u8,
    _pad1: u16,
    uid_high: u16,
    gid_high: u16,
    _reserved2: u32,
}

impl Into<FileAttributes> for Ext2InodeOnDisk {
    fn into(self) -> FileAttributes {
        FileAttributes {
            access: self.mode.into(),
            nlinks: self.num_links,
            uid: self.uid,
            gid: self.gid,
            rdev: None,
            inode: 0,
            size: self.size as usize,

            atime: self.atime.into(),
            mtime: self.mtime.into(),
            ctime: self.ctime.into(),
        }
    }
}

pub(super) struct Ext2Vnode {
    pub attrs: FileAttributes,
    pub mount: NonNull<Ext2Mount>,
    pub mounted_vnode: Option<Vnode>,
    pub blocks: [Ext2BlockNumber; EXT2_INODE_BLOCK_ENTRIES],
}

unsafe impl Send for Ext2Vnode {}
unsafe impl Sync for Ext2Vnode {}

fn get_mount(mount_ptr: NonNull<Ext2Mount>) -> &'static mut Ext2Mount {
    unsafe {
        &mut *mount_ptr.as_ptr()
    }
}

impl Ext2Vnode {
    fn new(mount_ptr: NonNull<Ext2Mount>) -> Self {
        Self {
            attrs: Default::default(),
            mount: mount_ptr,
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
        let vnode = self.vnode_cache.get(|node| node.lock().attributes().unwrap().inode == inode_num, || {
            let mut vnode = Ext2Vnode::new(mount_ptr);
            get_mount(mount_ptr).load_inode(&mut vnode, inode_num)?;
            Ok(Arc::new(Spinlock::new(vnode)))
        })?;

        Ok((*vnode).clone())
    }

    fn load_inode(&mut self, vnode: &mut Ext2Vnode, inode_num: Ext2InodeNum) -> Result<(), KernelError> {
        let (block_num, index) = self.superblock.get_inode_location(inode_num)?;
        let buf = block::get_buf(self.device_id, block_num)?;

        let data = unsafe {
            slice::from_raw_parts((buf.block.lock()).as_ptr() as *mut Ext2InodeOnDisk, self.superblock.inodes_per_block)
        };

        vnode.blocks = data[index].blocks;

        vnode.attrs = data[index].into();
        vnode.attrs.inode = inode_num;


        crate::printkln!("loaded inode {} {:#?}", inode_num, data[index]);
        Ok(())
    }
}


