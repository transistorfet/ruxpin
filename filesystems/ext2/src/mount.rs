
use ruxpin_types::DeviceID;

use ruxpin_kernel::block;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::misc::cache::Cache;

use ruxpin_kernel::fs::{MountOperations, Vnode, WeakVnode};

use super::Ext2InodeNum;
use super::superblock::Ext2SuperBlock;

pub(super) const EXT2_ROOT_INODE_NUM: Ext2InodeNum = 2;

pub struct Ext2Mount {
    pub(super) device_id: DeviceID,
    pub(super) mounted_on: Option<WeakVnode>,
    pub(super) superblock: Ext2SuperBlock,
    pub(super) vnode_cache: Cache<Ext2InodeNum, Vnode>,
}

impl Ext2Mount {
    pub(super) fn create_mount(parent: Option<WeakVnode>, device_id: DeviceID) -> Result<Ext2Mount, KernelError> {
        let superblock = Ext2SuperBlock::load(device_id)?;

        let mount = Ext2Mount {
            device_id,
            mounted_on: parent,
            superblock,
            vnode_cache: Cache::new(100),
        };

        Ok(mount)
    }
}

impl MountOperations for Ext2Mount {
    fn get_root(&mut self) -> Result<Vnode, KernelError> {
        self.get_inode(EXT2_ROOT_INODE_NUM)
    }

    fn sync(&mut self) -> Result<(), KernelError> {
        self.store_inodes()?;
        self.superblock.store()?;
        block::commit_all(self.device_id)?;
        Ok(())
    }

    fn unmount(&mut self) -> Result<(), KernelError> {
        self.superblock.store()?;
        block::close(self.device_id)?;
        Ok(())
    }
}

