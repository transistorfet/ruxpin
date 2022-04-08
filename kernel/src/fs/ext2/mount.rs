
use ruxpin_api::types::DeviceID;

use crate::block;
use crate::printkln;
use crate::errors::KernelError;
use crate::misc::cache::Cache;

use crate::fs::types::{MountOperations, Vnode, FilePointer};

use super::superblock::Ext2SuperBlock;

pub struct Ext2Mount {
    pub(super) device_id: DeviceID,
    pub(super) root_node: Option<Vnode>,
    pub(super) mounted_on: Option<Vnode>,
    pub(super) superblock: Ext2SuperBlock,
    pub(super) vnode_cache: Cache<Vnode>,
}

impl Ext2Mount {
    pub(super) fn create_mount(parent: Option<Vnode>, device_id: DeviceID) -> Result<Ext2Mount, KernelError> {
        let superblock = Ext2SuperBlock::load(device_id)?;

        let mut mount = Ext2Mount {
            device_id,
            root_node: None,
            mounted_on: parent,
            superblock,
            vnode_cache: Cache::new(100),
        };

        printkln!("superblock: {:#?}", mount.superblock);

        mount.root_node = Some(mount.get_inode(2)?);

        // TODO this is just a test
        if let Some(root_node) = mount.root_node.clone() {
            //let mut buffer = [0; 100];
            //let mut file = FilePointer::new(root_node.clone());
            //let result = root_node.lock().read(&mut file, &mut buffer)?;
            //printkln!("read {} bytes: {:?}", result, buffer);

            let mut file = FilePointer::new(root_node.clone());
            while let Some(entry) = root_node.lock().readdir(&mut file)? {
                printkln!("found {:?} at inode {}", entry.name.as_str(), entry.inode);
            }
        }

        Ok(mount)
    }
}

impl MountOperations for Ext2Mount {
    fn get_root(&self) -> Result<Vnode, KernelError> {
        match &self.root_node {
            Some(node) => Ok(node.clone()),
            None => Err(KernelError::NotAFile),
        }
    }

    fn sync(&mut self) -> Result<(), KernelError> {
        Ok(())
    }

    fn unmount(&mut self) -> Result<(), KernelError> {
        block::close(self.device_id)?;
        Ok(())
    }
}


