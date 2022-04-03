
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID, GroupID, DeviceID};

use crate::sync::Spinlock;
use crate::misc::StrArray;
use crate::errors::KernelError;

use super::types::{Filesystem, Mount, MountOperations, Vnode, VnodeOperations, FileAttributes, FilePointer, DirEntry};


const TMPFS_MAX_FILENAME: usize = 32;

pub struct TmpFilesystem {
    
}

pub struct TmpMount {
    root_node: Vnode,
    mounted_on: Option<Vnode>,
}

pub struct TmpDirEntry {
    name: StrArray<TMPFS_MAX_FILENAME>,
    vnode: Vnode,
}

pub struct TmpVnodeFile {
    attrs: FileAttributes,
    contents: Vec<u8>,
}

pub struct TmpVnodeDirectory {
    attrs: FileAttributes,
    contents: Vec<TmpDirEntry>,
    mounted_vnode: Option<Vnode>,
}

impl Filesystem for TmpFilesystem {
    fn fstype(&self) -> &'static str {
        "tmpfs"
    }

    fn init(&self) -> Result<(), KernelError> {

        Ok(())
    }

    fn mount(&mut self, parent: Option<Vnode>, _device_id: Option<DeviceID>) -> Result<Mount, KernelError> {
        let root_node = Arc::new(Spinlock::new(TmpVnodeDirectory::new(FileAccess::DefaultDir, 0, 0)));

        let mount = Arc::new(Spinlock::new(TmpMount {
            root_node,
            mounted_on: parent,
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

impl VnodeOperations for TmpVnodeDirectory {
    fn get_mount_mut<'a>(&'a mut self) -> Result<&'a mut Option<Vnode>, KernelError> {
        Ok(&mut self.mounted_vnode)
    }

    fn create(&mut self, filename: &str, access: FileAccess, current_uid: UserID) -> Result<Vnode, KernelError> {
        let entry = TmpDirEntry::new(filename, access, current_uid);
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


    // TODO add link
    // TODO add unlink
    // TODO add rename


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

    fn readdir(&mut self, file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {
        if file.position >= self.contents.len() {
            return Ok(None);
        }

        let result = DirEntry {
            inode: 0,
            name: self.contents[file.position].name.as_str().try_into()?,
        };

        file.position += 1;

        Ok(Some(result))
    }
}

impl VnodeOperations for TmpVnodeFile {
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

impl TmpDirEntry {
    pub fn new(name: &str, access: FileAccess, uid: UserID) -> Self {
        let vnode: Vnode = if access.is_dir() {
            Arc::new(Spinlock::new(TmpVnodeDirectory::new(access, uid, 0)))
        } else {
            Arc::new(Spinlock::new(TmpVnodeFile::new(access, uid, 0)))
        };

        Self {
            name: name.try_into().unwrap(),
            vnode,
        }
    }
}

impl TmpVnodeDirectory {
    pub fn new(access: FileAccess, uid: UserID, gid: GroupID) -> Self {
        Self {
            attrs: FileAttributes::new(access, uid, gid),
            contents: Vec::new(),
            mounted_vnode: None,
        }
    }
}

impl TmpVnodeFile {
    pub fn new(access: FileAccess, uid: UserID, gid: GroupID) -> Self {
        Self {
            attrs: FileAttributes::new(access, uid, gid),
            contents: Vec::new(),
        }
    }
}

