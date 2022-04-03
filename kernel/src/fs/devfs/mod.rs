
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID, DeviceID};

use crate::tty;
use crate::sync::Spinlock;
use crate::misc::StrArray;
use crate::errors::KernelError;

use super::types::{Filesystem, Mount, MountOperations, Vnode, VnodeOperations, FileAttributes, FilePointer, DirEntry};


const DEVFS_MAX_FILENAME: usize = 32;

pub struct DevFilesystem {
    
}

pub struct DevMount {
    root_node: Vnode,
    mounted_on: Option<Vnode>,
}

pub struct DevDirEntry {
    name: StrArray<DEVFS_MAX_FILENAME>,
    vnode: Vnode,
}

pub struct DevVnodeDirectory {
    attrs: FileAttributes,
    contents: Vec<DevDirEntry>,
}

pub struct DevVnodeRootDirectory {
    attrs: FileAttributes,
}

pub struct DevVnodeCharDevice {
    attrs: FileAttributes,
    device_id: DeviceID,
}


impl Filesystem for DevFilesystem {
    fn fstype(&self) -> &'static str {
        "devfs"
    }

    fn init(&self) -> Result<(), KernelError> {

        Ok(())
    }

    fn mount(&mut self, parent: Option<Vnode>, _device_id: Option<DeviceID>) -> Result<Mount, KernelError> {
        let root_node = Arc::new(Spinlock::new(DevVnodeRootDirectory::new()));

        let mount = Arc::new(Spinlock::new(DevMount {
            root_node,
            mounted_on: parent,
        }));

        Ok(mount)
    }
}

impl DevFilesystem {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl MountOperations for DevMount {
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

impl VnodeOperations for DevVnodeDirectory {
    fn create(&mut self, filename: &str, access: FileAccess, uid: UserID) -> Result<Vnode, KernelError> {
        let entry = DevDirEntry::try_new(filename, access)?;
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

    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Ok(&mut self.attrs)
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

        let result = DirEntry {
            inode: 0,
            name: self.contents[file.position].name.as_str().try_into()?,
        };

        file.position += 1;

        Ok(Some(result))
    }
}

impl VnodeOperations for DevVnodeRootDirectory {
    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        // TODO will need to support block devices as well
        let device_id = tty::lookup_device(filename)?;
        Ok(Arc::new(Spinlock::new(DevVnodeCharDevice::new(device_id))))
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

    //fn readdir(&mut self, file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {
    //
    //}
}

impl VnodeOperations for DevVnodeCharDevice {
    fn open(&mut self, _file: &mut FilePointer, flags: OpenFlags) -> Result<(), KernelError> {
        tty::open(self.device_id, flags)
    }

    fn close(&mut self, _file: &mut FilePointer) -> Result<(), KernelError> {
        tty::close(self.device_id)
    }

    fn read(&mut self, file: &mut FilePointer, buffer: &mut [u8]) -> Result<usize, KernelError> {
        let nbytes = tty::read(self.device_id, buffer)?;
        file.position += nbytes;
        Ok(nbytes)
    }

    fn write(&mut self, file: &mut FilePointer, buffer: &[u8]) -> Result<usize, KernelError> {
        let nbytes = tty::write(self.device_id, buffer)?;
        file.position += nbytes;
        Ok(nbytes)
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

impl DevVnodeDirectory {
    pub fn new() -> Self {
        Self {
            attrs: Default::default(),
            contents: Vec::new(),
        }
    }
}

impl DevVnodeRootDirectory {
    pub fn new() -> Self {
        Self {
            attrs: Default::default(),
        }
    }
}

impl DevVnodeCharDevice {
    pub fn new(device_id: DeviceID) -> Self {
        Self {
            attrs: Default::default(),
            device_id,
        }
    }
}

impl DevDirEntry {
    pub fn try_new(name: &str, access: FileAccess) -> Result<Self, KernelError> {
        let vnode = if access.is_dir() {
            Arc::new(Spinlock::new(DevVnodeDirectory::new()))
        } else {
            return Err(KernelError::OperationNotPermitted);
        };

        let mut array = StrArray::new();
        array.copy_into(name);
        Ok(Self {
            name: array,
            vnode,
        })
    }
}

