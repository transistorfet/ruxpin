#![no_std]

extern crate alloc;

use alloc::sync::Arc;

use ruxpin_types::{OpenFlags, FileAccess, Seek, DeviceID};

use ruxpin_kernel::tty;
use ruxpin_kernel::sync::Spinlock;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::fs::types::{new_vnode, Filesystem, Mount, MountOperations, Vnode, WeakVnode, VnodeOperations, FileAttributes, FilePointer};


pub struct DevFilesystem {
    
}

pub struct DevMount {
    root_node: Vnode,
}

pub struct DevRootDirectoryVnode {
    self_vnode: Option<WeakVnode>,
    parent_vnode: Option<WeakVnode>,
    attrs: FileAttributes,
}

pub struct DevCharDeviceVnode {
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

    fn mount(&mut self, parent: Option<WeakVnode>, _device_id: Option<DeviceID>) -> Result<Mount, KernelError> {
        let root_node = Arc::new(Spinlock::new(DevRootDirectoryVnode::new(parent)));

        let mount = Arc::new(Spinlock::new(DevMount {
            root_node,
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

impl DevRootDirectoryVnode {
    pub fn new(parent_vnode: Option<WeakVnode>) -> Self {
        Self {
            self_vnode: None,
            parent_vnode,
            attrs: FileAttributes::new(FileAccess::DefaultDir, 0, 0),
        }
    }
}

impl DevCharDeviceVnode {
    pub fn new(device_id: DeviceID) -> Self {
        Self {
            attrs: Default::default(),
            device_id,
        }
    }
}

impl VnodeOperations for DevRootDirectoryVnode {
    fn set_self(&mut self, vnode: WeakVnode) {
        self.self_vnode = Some(vnode);
    }

    fn lookup(&mut self, filename: &str) -> Result<Vnode, KernelError> {
        let weak_ref = if filename == "." {
            self.self_vnode.as_ref()
        } else if filename == ".." {
            self.parent_vnode.as_ref()
        } else {
            None
        };

        if let Some(vnode) = weak_ref {
            return vnode.upgrade().ok_or(KernelError::FileNotFound);
        }

        // TODO will need to support block devices as well
        let device_id = tty::lookup_device(filename)?;
        Ok(new_vnode(DevCharDeviceVnode::new(device_id)))
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

    //fn readdir(&mut self, file: &mut FilePointer) -> Result<Option<DirEntry>, KernelError> {
    //
    //}
}

impl VnodeOperations for DevCharDeviceVnode {
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


