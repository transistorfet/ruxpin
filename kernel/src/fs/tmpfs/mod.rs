
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID};

use crate::sync::Spinlock;
use crate::misc::StrArray;
use crate::errors::KernelError;

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
    pub fn new(name: &str, access: FileAccess) -> Self {
        let vnode = if access.is_dir() {
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
    fn create(&mut self, filename: &str, access: FileAccess, uid: UserID) -> Result<Vnode, KernelError> {
        let contents = self.as_dir()?;

        let entry = TmpDirEntry::new(filename, access);
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
        Ok(&mut self.attrs)
    }

    //fn attributes_mut<'a>(&'a mut self) -> Result<&'a mut FileAttributes, KernelError> {
    //    // TODO this isn't right because you need to update
    //    Ok(&mut self.attrs)
    //}

    fn open(&mut self, _file: &mut FilePointer, _flags: OpenFlags) -> Result<(), KernelError> {
        Ok(())
    }

    fn close(&mut self, _file: &mut FilePointer) -> Result<(), KernelError> {
        Ok(())
    }

    fn read(&mut self, file: &mut FilePointer, buffer: &mut [u8]) -> Result<usize, KernelError> {
        let data = self.as_file()?;

        let start = file.position;
        for byte in buffer {
            if file.position >= data.len() {
                break;
            }
            *byte = data[file.position];
            file.position += 1;
        }
        Ok(file.position - start)
    }

    fn write(&mut self, file: &mut FilePointer, buffer: &[u8]) -> Result<usize, KernelError> {
        let data = self.as_file()?;

        let start = file.position;
        for byte in buffer {
            if file.position >= data.len() {
                for _ in data.len()..=file.position {
                    data.push(0);
                }
            }
            data[file.position] = *byte;
            file.position += 1;
        }
        Ok(file.position - start)
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

    pub fn as_file<'a>(&'a mut self) -> Result<&'a mut Vec<u8>, KernelError> {
        if let TmpVnodeContents::File(data) = &mut self.contents {
            Ok(data)
        } else {
            Err(KernelError::NotFile)
        }
    }
}

