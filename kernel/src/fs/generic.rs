
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::string::String;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID, GroupID, DirEntry};

use crate::sync::Spinlock;
use crate::errors::KernelError;

use super::types::{Vnode, VnodeOperations, FileAttributes, FilePointer};


pub struct GenericDirEntry {
    name: String,
    vnode: Vnode,
}

impl GenericDirEntry {
    pub fn new(name: &str, access: FileAccess, uid: UserID, gid: GroupID) -> Self {
        let vnode: Vnode = if access.is_dir() {
            Arc::new(Spinlock::new(GenericDirectoryVnode::new(access, uid, gid)))
        } else {
            Arc::new(Spinlock::new(GenericFileVnode::empty(access, uid, gid)))
        };

        Self {
            name: name.try_into().unwrap(),
            vnode,
        }
    }
}

pub struct GenericDirectoryVnode {
    attrs: FileAttributes,
    contents: Vec<GenericDirEntry>,
    mounted_vnode: Option<Vnode>,
}

impl GenericDirectoryVnode {
    pub fn new(access: FileAccess, uid: UserID, gid: GroupID) -> Self {
        Self {
            attrs: FileAttributes::new(access, uid, gid),
            contents: Vec::new(),
            mounted_vnode: None,
        }
    }
}

impl VnodeOperations for GenericDirectoryVnode {
    fn get_mount_mut<'a>(&'a mut self) -> Result<&'a mut Option<Vnode>, KernelError> {
        Ok(&mut self.mounted_vnode)
    }

    fn create(&mut self, filename: &str, access: FileAccess, uid: UserID, gid: GroupID) -> Result<Vnode, KernelError> {
        let entry = GenericDirEntry::new(filename, access, uid, gid);
        let vnode = entry.vnode.clone();
        self.contents.push(entry);
        Ok(vnode)
    }

    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        for entry in &self.contents {
            if entry.name.as_str() == filename {
                return Ok(entry.vnode.clone());
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

    fn readdir(&mut self, file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {
        if file.position >= self.contents.len() {
            return Ok(None);
        }

        let result = DirEntry::new(0, self.contents[file.position].name.as_str().as_bytes());

        file.position += 1;

        Ok(Some(result))
    }
}


pub struct GenericFileVnode {
    attrs: FileAttributes,
    contents: Vec<u8>,
}

impl GenericFileVnode {
    pub fn empty(access: FileAccess, uid: UserID, gid: GroupID) -> Self {
        Self {
            attrs: FileAttributes::new(access, uid, gid),
            contents: Vec::new(),
        }
    }

    pub fn with_data(access: FileAccess, uid: UserID, gid: GroupID, data: Vec<u8>) -> Self {
        Self {
            attrs: FileAttributes::new(access, uid, gid),
            contents: data,
        }
    }
}

impl VnodeOperations for GenericFileVnode {
    fn truncate(&mut self) -> Result<(), KernelError> {
        self.contents.clear();
        Ok(())
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
        let start = file.position;
        for byte in buffer {
            if file.position >= self.contents.len() {
                break;
            }
            *byte = self.contents[file.position];
            file.position += 1;
        }
        Ok(file.position - start)
    }

    fn write(&mut self, file: &mut FilePointer, buffer: &[u8]) -> Result<usize, KernelError> {
        let start = file.position;
        for byte in buffer {
            if file.position >= self.contents.len() {
                for _ in self.contents.len()..=file.position {
                    self.contents.push(0);
                }
            }
            self.contents[file.position] = *byte;
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

pub struct GenericStaticDirectoryVnode<T: 'static + Sync + Send> {
    attrs: FileAttributes,
    contents: &'static [(&'static str, GenericStaticFileData<T>)],
    data: T,
}

pub type GenericStaticFileData<T> = fn(&T) -> Result<Vec<u8>, KernelError>;

impl<T: 'static + Sync + Send> GenericStaticDirectoryVnode<T> {
    pub fn new(access: FileAccess, uid: UserID, gid: GroupID, contents: &'static [(&'static str, GenericStaticFileData<T>)], data: T) -> Self {
        Self {
            attrs: FileAttributes::new(access, uid, gid),
            contents,
            data,
        }
    }

    fn get_name(&self, pos: usize) -> &'static str {
        self.contents[pos].0
    }

    fn get_data_by_name(&self, filename: &str) -> Result<Vec<u8>, KernelError> {
        for (name, func) in self.contents {
            if *name == filename {
                return func(&self.data);
            }
        }
        Err(KernelError::FileNotFound)
    }
}

impl<T: 'static + Sync + Send> VnodeOperations for GenericStaticDirectoryVnode<T> {
    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Ok(&mut self.attrs)
    }

    //fn attributes_mut(&mut self, f: &mut dyn FnMut(&mut FileAttributes)) -> Result<(), KernelError> {
    //    Err(KernelError::OperationNotPermitted)
    //}

    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        let data = self.get_data_by_name(filename)?;
        Ok(Arc::new(Spinlock::new(GenericFileVnode::with_data(FileAccess::DefaultReadOnlyFile, 0, 0, data))))
    }

    fn open(&mut self, _file: &mut FilePointer, _flags: OpenFlags) -> Result<(), KernelError> {
        Ok(())
    }

    fn close(&mut self, _file: &mut FilePointer) -> Result<(), KernelError> {
        Ok(())
    }

    fn readdir(&mut self, file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {
        if file.position >= self.contents.len() {
            return Ok(None);
        }

        let result = DirEntry::new(0, self.get_name(file.position).as_bytes());

        file.position += 1;
        Ok(Some(result))
    }
}

