#![no_std]

extern crate alloc;

use alloc::sync::Arc;

use ruxpin_api::types::{FileAccess, DeviceID};

use ruxpin_kernel::sync::Spinlock;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::fs::generic::GenericDirectoryVnode;
use ruxpin_kernel::fs::types::{Filesystem, Mount, MountOperations, Vnode};


pub struct TmpFilesystem {
    
}

pub struct TmpMount {
    root_node: Vnode,
    mounted_on: Option<Vnode>,
}

impl Filesystem for TmpFilesystem {
    fn fstype(&self) -> &'static str {
        "tmpfs"
    }

    fn init(&self) -> Result<(), KernelError> {

        Ok(())
    }

    fn mount(&mut self, parent: Option<Vnode>, _device_id: Option<DeviceID>) -> Result<Mount, KernelError> {
        let root_node = Arc::new(Spinlock::new(GenericDirectoryVnode::new(FileAccess::DefaultDir, 0, 0)));

        let mount = Arc::new(Spinlock::new(TmpMount {
            root_node,
            mounted_on: parent,
        }));

        Ok(mount)
    }
}

impl TmpFilesystem {
    pub fn new() -> Arc<Spinlock<dyn Filesystem>> {
        Arc::new(Spinlock::new(Self {

        }))
    }
}

impl MountOperations for TmpMount {
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


