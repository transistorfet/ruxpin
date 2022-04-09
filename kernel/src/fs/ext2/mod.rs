
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID, DeviceID};

use crate::block;
use crate::printkln;
use crate::sync::Spinlock;
use crate::errors::KernelError;

use super::types::{Filesystem, Mount, Vnode, VnodeOperations, FileAttributes, FilePointer, DirEntry};

mod inodes;
mod files;
mod directories;
mod mount;
mod superblock;

use self::inodes::{Ext2Vnode, Ext2InodeNum};
use self::mount::Ext2Mount;


pub struct Ext2Filesystem {
    /* Nothing For The Moment */
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

        let mount = Ext2Mount::create_mount(parent, device_id)?;
        printkln!("superblock: {:#?}", mount.superblock);

        Ok(Arc::new(Spinlock::new(mount)))
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
    */

    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        if !self.attrs.access.is_dir() {
            return Err(KernelError::NotADirectory);
        }

        let mut position = 0;
        let mut dirent = DirEntry::new();
        while position < self.attrs.size {
            position += self.read_directory_from_vnode(&mut dirent, position)?;
            printkln!("found {:?} at inode {}", dirent.name.as_str(), dirent.inode);
            if dirent.name.as_str() == filename {
                printkln!("a winner: inode {}", dirent.inode);
                return self.get_inode(dirent.inode);
            }
        }
        Err(KernelError::FileNotFound)
    }

    //fn link(&mut self, _newparent: Vnode, _filename: &str) -> Result<Vnode, KernelError> {
    //    Err(KernelError::OperationNotPermitted)
    //}

    //fn unlink(&mut self, _target: Vnode, _filename: &str) -> Result<Vnode, KernelError> {
    //    Err(KernelError::OperationNotPermitted)
    //}

    //fn rename(&mut self, _filename: &str) -> Result<Vnode, KernelError> {
    //    Err(KernelError::OperationNotPermitted)
    //}

    /*
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

    //fn attributes_mut(&mut self, f: &mut dyn FnMut(&mut FileAttributes)) -> Result<(), KernelError> {
    //    Err(KernelError::OperationNotPermitted)
    //}

    fn open(&mut self, _file: &mut FilePointer, _flags: OpenFlags) -> Result<(), KernelError> {
        Ok(())
    }

    fn close(&mut self, _file: &mut FilePointer) -> Result<(), KernelError> {
        Ok(())
    }

    fn read(&mut self, file: &mut FilePointer, buffer: &mut [u8]) -> Result<usize, KernelError> {
        if self.attrs.access.is_dir() {
            return Err(KernelError::IsADirectory);
        }

	let nbytes = if buffer.len() > self.attrs.size - file.position {
	    self.attrs.size - file.position
        } else {
            buffer.len()
        };

        let offset = self.read_from_vnode(buffer, nbytes, file.position)?;

	file.position += offset;
	Ok(offset)
    }

    /*
    fn write(&mut self, file: &mut FilePointer, buffer: &[u8]) -> Result<usize, KernelError> {

    }
    */

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
        if !self.attrs.access.is_dir() {
            return Err(KernelError::NotADirectory);
        }

        if file.position >= self.attrs.size {
            Ok(None)
        } else {
            let mut dirent = DirEntry::new();
            let offset = self.read_directory_from_vnode(&mut dirent, file.position)?;

            file.position += offset;
            Ok(Some(dirent))
        }
    }
}
