
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::{FileFlags, FileAccess, Seek, UserID};

use crate::misc::StrArray;
use crate::errors::KernelError;
use crate::arch::sync::Spinlock;

use super::types::{Filesystem, Mount, MountOperations, Vnode, VnodeOperations, FileAttributes, FilePointer};


const TMPFS_MAX_FILENAME: usize = 14;

pub struct TmpFilesystem {
    
}

pub struct TmpMount {
    root_node: Vnode,
}

pub struct TmpDirEntry {
    name: StrArray<TMPFS_MAX_FILENAME>,
    vnode: Vnode,
}

pub enum TmpVnodeContents {
    Directory(Vec<TmpDirEntry>),
    File(Vec<u8>),
}

pub struct TmpVnode {
    attrs: FileAttributes,
    contents: TmpVnodeContents,
}

impl Filesystem for TmpFilesystem {
    fn fstype(&self) -> &'static str {
        "tmpfs"
    }

    fn init(&self) -> Result<(), KernelError> {

        Ok(())
    }

    fn mount(&mut self) -> Result<Mount, KernelError> {
        let root_node = Arc::new(Spinlock::new(TmpVnode::new_directory()));

        let mount = Arc::new(Spinlock::new(TmpMount {
            root_node,
        }));

        Ok(mount)
    }
}

impl TmpFilesystem {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl MountOperations for TmpMount {
    fn get_root(&self) -> Result<Vnode, KernelError> {
        Ok(self.root_node.clone())
    }

    fn sync(&mut self) -> Result<(), KernelError> {
        Ok(())
    }

    fn unmount(&mut self) -> Result<(), KernelError> {
        Ok(())
    }
}

impl TmpDirEntry {
    pub fn new(name: &str, mode: FileAccess) -> Self {
        let vnode = if mode.is_dir() {
            Arc::new(Spinlock::new(TmpVnode::new_directory()))
        } else {
            Arc::new(Spinlock::new(TmpVnode::new_file()))
        };

        let mut array = StrArray::new();
        array.copy_into(name);
        Self {
            name: array,
            vnode,
        }
    }
}

impl VnodeOperations for TmpVnode {
    fn create(&mut self, filename: &str, mode: FileAccess, uid: UserID) -> Result<Vnode, KernelError> {
        let contents = self.as_dir()?;

        let entry = TmpDirEntry::new(filename, mode);
        let vnode = entry.vnode.clone();
        contents.push(entry);
        Ok(vnode)
    }

    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        let contents = self.as_dir()?;

        for entry in contents {
            if entry.name.as_str() == filename {
                return Ok(entry.vnode.clone());
            }
        }
        Err(KernelError::FileNotFound)
    }

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Ok(&self.attrs)
    }

    //fn attributes_mut<'a>(&'a mut self) -> Result<&'a mut FileAttributes, KernelError> {
    //    // TODO this isn't right because you need to update
    //    Ok(&mut self.attrs)
    //}

    fn open(&self, file: &mut FilePointer, mode: FileFlags) -> Result<(), KernelError> {
        Ok(())
    }

    fn close(&self, file: &mut FilePointer) -> Result<(), KernelError> {
        Ok(())
    }

    fn read(&self, file: &mut FilePointer, buffer: &mut [u8]) -> Result<usize, KernelError> {
        Ok(0)
    }

    fn write(&self, file: &mut FilePointer, buffer: &[u8]) -> Result<usize, KernelError> {
        Ok(0)
    }

    fn seek(&self, file: &mut FilePointer, position: usize, whence: Seek) -> Result<usize, KernelError> {
        Ok(0)
    }
}

impl TmpVnode {
    pub fn new_directory() -> Self {
        Self {
            attrs: Default::default(),
            contents: TmpVnodeContents::Directory(Vec::new()),
        }
    }

    pub fn new_file() -> Self {
        Self {
            attrs: Default::default(),
            contents: TmpVnodeContents::File(Vec::new()),
        }
    }

    pub fn as_dir<'a>(&'a mut self) -> Result<&'a mut Vec<TmpDirEntry>, KernelError> {
        if let TmpVnodeContents::Directory(list) = &mut self.contents {
            Ok(list)
        } else {
            Err(KernelError::NotDirectory)
        }
    }
}

