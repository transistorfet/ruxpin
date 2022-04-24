
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::{OpenFlags, FileAccess, Seek, UserID, DeviceID, GroupID, DirEntry};

use crate::tty;
use crate::sync::Spinlock;
use crate::misc::strarray::StrArray;
use crate::errors::KernelError;

use super::types::{Filesystem, Mount, MountOperations, Vnode, VnodeOperations, FileAttributes, FilePointer};


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
    pub fn new() -> Arc<Spinlock<dyn Filesystem>> {
        Arc::new(Spinlock::new(Self {

        }))
    }
}

impl MountOperations for DevMount {
    fn get_root(&mut self) -> Result<Vnode, KernelError> {
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
    fn create(&mut self, filename: &str, access: FileAccess, uid: UserID, _gid: GroupID) -> Result<Vnode, KernelError> {
        let entry = DevDirEntry::try_new(filename, access, uid)?;
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

        let result = DirEntry::new(0, self.contents[file.position].name.as_str().as_bytes());

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

    //fn attributes_mut(&mut self, f: &mut dyn FnMut(&mut FileAttributes)) -> Result<(), KernelError> {
    //    Err(KernelError::OperationNotPermitted)
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
    fn attributes<'a>(&'a mut self) -> Result<&'a FileAttributes, KernelError> {
        Ok(&mut self.attrs)
    }

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
    pub fn new(access: FileAccess, uid: UserID, gid: GroupID) -> Self {
        Self {
            attrs: FileAttributes::new(access, uid, gid),
            contents: Vec::new(),
        }
    }
}

impl DevVnodeRootDirectory {
    pub fn new() -> Self {
        Self {
            attrs: FileAttributes::new(FileAccess::DefaultDir, 0, 0),
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
    pub fn try_new(name: &str, access: FileAccess, uid: UserID) -> Result<Self, KernelError> {
        let vnode = if access.is_dir() {
            Arc::new(Spinlock::new(DevVnodeDirectory::new(access, uid, 0)))
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

