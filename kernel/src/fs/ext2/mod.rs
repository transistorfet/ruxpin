
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID, DeviceID};

use crate::block;
use crate::printkln;
use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::misc::cache::Cache;

use super::types::{Filesystem, Mount, MountOperations, Vnode, VnodeOperations, FileAttributes, FilePointer, DirEntry};

mod inodes;
mod superblock;

use self::inodes::{Ext2Vnode, Ext2InodeNum};
use self::superblock::Ext2SuperBlock;


pub struct Ext2Filesystem {
    /* Nothing For The Moment */
}

pub struct Ext2Mount {
    device_id: DeviceID,
    root_node: Option<Vnode>,
    mounted_on: Option<Vnode>,
    superblock: Ext2SuperBlock,
    vnode_cache: Cache<Vnode>,
}

impl Ext2Filesystem {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl Filesystem for Ext2Filesystem {
    fn fstype(&self) -> &'static str {
        "ext2"
    }

    fn init(&self) -> Result<(), KernelError> {

        Ok(())
    }

    fn mount(&mut self, parent: Option<Vnode>, device_id: Option<DeviceID>) -> Result<Mount, KernelError> {
        let device_id = device_id.ok_or(KernelError::NoSuchDevice)?;
        block::open(device_id, OpenFlags::ReadOnly)?;

        let superblock = Ext2SuperBlock::load(device_id)?;

        let mut mount = Ext2Mount {
            device_id,
            root_node: None,
            mounted_on: parent,
            superblock,
            vnode_cache: Cache::new(100),
        };

        printkln!("superblock: {:#?}", mount.superblock);

        mount.root_node = Some(mount.get_inode(0)?);

        Ok(Arc::new(Spinlock::new(mount)))
    }
}

impl MountOperations for Ext2Mount {
    fn get_root(&self) -> Result<Vnode, KernelError> {
        match &self.root_node {
            Some(node) => Ok(node.clone()),
            None => Err(KernelError::NotFile),
        }
    }

    fn sync(&mut self) -> Result<(), KernelError> {
        Ok(())
    }

    fn unmount(&mut self) -> Result<(), KernelError> {
        block::close(self.device_id)?;
        Ok(())
    }
}

impl VnodeOperations for Ext2Vnode {
    fn get_mount_mut<'a>(&'a mut self) -> Result<&'a mut Option<Vnode>, KernelError> {
        Ok(&mut self.mounted_vnode)
    }

    /*
    fn create(&mut self, filename: &str, access: FileAccess, uid: UserID) -> Result<Vnode, KernelError> {
        if !self.attrs.access.is_dir() {
            return Err(KernelError::OperationNotPermitted);
        }


        Ok(vnode)
    }

    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        if !self.attrs.access.is_dir() {
            return Err(KernelError::OperationNotPermitted);
        }


        Err(KernelError::FileNotFound)
    }

    // TODO add link
    // TODO add unlink
    // TODO add rename

    fn truncate(&mut self) -> Result<(), KernelError> {
        if self.attrs.access.is_file() {

        } else {
            //self.contents.clear();
        }
        Ok(())
    }
    */

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Ok(&mut self.attrs)
    }

    //fn attributes_mut<'a>(&'a mut self) -> Result<&'a mut FileAttributes, KernelError> {
    //    // TODO this isn't right because you need to update
    //    Ok(&mut self.attrs)
    //}

    /*
    fn open(&mut self, _file: &mut FilePointer, _flags: OpenFlags) -> Result<(), KernelError> {
        Ok(())
    }

    fn close(&mut self, _file: &mut FilePointer) -> Result<(), KernelError> {
        Ok(())
    }

    fn read(&mut self, file: &mut FilePointer, buffer: &mut [u8]) -> Result<usize, KernelError> {

    }

    fn write(&mut self, file: &mut FilePointer, buffer: &[u8]) -> Result<usize, KernelError> {

    }

    fn seek(&mut self, file: &mut FilePointer, offset: usize, whence: Seek) -> Result<usize, KernelError> {
        let position = match whence {
            Seek::FromStart => offset,
            Seek::FromCurrent => file.position + offset,
            Seek::FromEnd => self.attrs.size + offset,
        };

        if position >= self.attrs.size {
            file.position = self.attrs.size;
        } else {
            file.position = position;
        }
        Ok(file.position)
    }

    fn readdir(&mut self, file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {

    }
    */
}

