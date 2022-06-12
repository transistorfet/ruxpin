#![no_std]

extern crate alloc;

use alloc::sync::Arc;

use ruxpin_types::{OpenFlags, FileAccess, Seek, DeviceID, UserID, GroupID, DirEntry};

use ruxpin_kernel::trace;
use ruxpin_kernel::block;
use ruxpin_kernel::sync::Spinlock;
use ruxpin_kernel::errors::KernelError;

use ruxpin_kernel::fs::vfs;
use ruxpin_kernel::fs::types::{Filesystem, Mount, Vnode, VnodeOperations, FileAttributes, FilePointer};

mod blocks;
mod directories;
mod files;
mod inodes;
mod mount;
mod superblock;

pub(self) type Ext2InodeNum = u32;
pub(self) type Ext2BlockNumber = u32;

use self::mount::Ext2Mount;
use self::inodes::Ext2Vnode;


pub struct Ext2Filesystem {
    /* Nothing For The Moment */
}

impl Ext2Filesystem {
    pub fn new() -> Arc<Spinlock<dyn Filesystem>> {
        Arc::new(Spinlock::new(Self {

        }))
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
        //debug!("superblock: {:#?}", mount.superblock);

        Ok(Arc::new(Spinlock::new(mount)))
    }
}

impl VnodeOperations for Ext2Vnode {
    fn get_mount_mut<'a>(&'a mut self) -> Result<&'a mut Option<Vnode>, KernelError> {
        Ok(&mut self.mounted_vnode)
    }

    fn commit(&mut self) -> Result<(), KernelError> {
        self.writeback()
    }

    fn create(&mut self, filename: &str, access: FileAccess, uid: UserID, gid: GroupID) -> Result<Vnode, KernelError> {
        if !self.attrs.access.is_dir() {
            return Err(KernelError::OperationNotPermitted);
        }

        let (inode_num, vnode) = self.get_mount().alloc_inode(self.attrs.inode, access, uid, gid)?;
        self.add_directory_to_vnode(filename, inode_num, access)?;

        Ok(vnode)
    }

    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        if !self.attrs.access.is_dir() {
            return Err(KernelError::NotADirectory);
        }

        let mut position = 0;
        let mut dirent = DirEntry::new_empty();
        while position < self.attrs.size {
            let nbytes = match self.read_next_dirent_from_vnode(&mut dirent, position)? {
                None => { break; },
                Some(nbtyes) => nbtyes,
            };
            position += nbytes;
            if dirent.as_str() == filename {
                trace!("ext2: looking for {:?}, found inode {}", filename, dirent.inode);
                return self.get_inode(dirent.inode);
            }
        }
        Err(KernelError::FileNotFound)
    }

    fn link(&mut self, target: Vnode, filename: &str) -> Result<(), KernelError> {
        self.add_directory_to_vnode(filename, target.lock().attributes()?.inode, self.attrs.access)?;
        target.lock().attributes_mut(&mut |attrs| {
            attrs.nlinks += 1;
        })?;
        Ok(())
    }

    fn unlink(&mut self, target: Vnode, filename: &str) -> Result<(), KernelError> {
        if vfs::is_directory(target.clone())? && !vfs::is_directory_empty(target.clone())? {
            return Err(KernelError::DirectoryNotEmpty);
        }

        let inode = self.remove_directory_entry(filename)?;
        self.dirty = true;
        self.attrs.nlinks -= 1;
        if self.attrs.nlinks == 0 {
            target.try_lock().unwrap().truncate()?;
            self.free_inode(inode)?;
        }
        Ok(())
    }

    fn rename(&mut self, old_name: &str, new_parent: Option<Vnode>, new_name: &str) -> Result<(), KernelError> {
        let target = self.lookup(old_name)?;
        if let Some(new_parent) = new_parent {
            new_parent.lock().link(target, new_name)?;
        } else {
            self.link(target, new_name)?;
        }

        self.remove_directory_entry(old_name)?;
        self.attrs.nlinks -= 1;
        self.dirty = true;
        Ok(())
    }

    fn truncate(&mut self) -> Result<(), KernelError> {
        self.free_all_blocks()?;
        Ok(())
    }

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Ok(&mut self.attrs)
    }

    fn attributes_mut(&mut self, func: &mut dyn FnMut(&mut FileAttributes)) -> Result<(), KernelError> {
        func(&mut self.attrs);
        self.dirty = true;
        Ok(())
    }

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

    fn write(&mut self, file: &mut FilePointer, buffer: &[u8]) -> Result<usize, KernelError> {
        if self.attrs.access.is_dir() {
            return Err(KernelError::IsADirectory);
        }

        let offset = self.write_to_vnode(buffer, buffer.len(), file.position)?;

        file.position += offset;
        if file.position > self.attrs.size {
            self.attrs.size = file.position;
        }
        self.writeback()?;

        Ok(offset)
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
        if !self.attrs.access.is_dir() {
            return Err(KernelError::NotADirectory);
        }

        if file.position >= self.attrs.size {
            Ok(None)
        } else {
            let mut dirent = DirEntry::new_empty();
            match self.read_next_dirent_from_vnode(&mut dirent, file.position)? {
                None => Ok(None),
                Some(offset) => {
                    file.position += offset;
                    Ok(Some(dirent))
                },
            }
        }
    }
}

