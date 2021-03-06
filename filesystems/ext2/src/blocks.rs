
use core::mem;

use ruxpin_types::DeviceID;

use ruxpin_kernel::block;
use ruxpin_kernel::misc::memory;
use ruxpin_kernel::block::BlockNum;
use ruxpin_kernel::errors::KernelError;

use super::Ext2InodeNum;
use super::Ext2BlockNumber;
use super::mount::Ext2Mount;
use super::inodes::{Ext2Vnode, EXT2_INODE_DIRECT_BLOCKS};


#[derive(Copy, Clone, PartialEq)]
pub(super) enum GetFileBlockOp {
    Lookup,
    Allocate,
}

impl Ext2Vnode {
    pub fn get_file_block_num(&mut self, linear_block_num: usize, op: GetFileBlockOp) -> Result<Option<BlockNum>, KernelError> {
        let tiers = self.get_number_of_tiers(linear_block_num)?;
        let index = if linear_block_num < EXT2_INODE_DIRECT_BLOCKS {
            linear_block_num
        } else {
            EXT2_INODE_DIRECT_BLOCKS + tiers - 1
        };

        if self.blocks[index] == 0 {
            if op == GetFileBlockOp::Lookup {
                return Ok(None);
            }
            self.blocks[index] = self.get_mount().alloc_block(self.attrs.inode as Ext2InodeNum)?;
        }

        if tiers == 0 {
            Ok(Some(self.blocks[index]))
        } else {
            self.get_block_in_tier(self.get_device_id(), tiers, self.blocks[index], linear_block_num - EXT2_INODE_DIRECT_BLOCKS, op)
        }
    }

    fn get_block_in_tier(&mut self, device_id: DeviceID, tiers: usize, table_block: BlockNum, offset: usize, op: GetFileBlockOp) -> Result<Option<BlockNum>, KernelError> {
        let entries_per_block = self.get_block_size() / mem::size_of::<Ext2BlockNumber>();
        let index = offset / entries_per_block.pow(tiers as u32 - 1);

        if tiers <= 1 {
            self.get_or_allocate_block(device_id, table_block, index, op)
        } else {
            let block = match self.get_or_allocate_block(device_id, table_block, index, op)? {
                None => { return Ok(None); },
                Some(block) => block,
            };
            let remain = offset % entries_per_block.pow(tiers as u32 - 1);
            self.get_block_in_tier(device_id, tiers - 1, block, remain, op)
        }
    }

    fn get_or_allocate_block(&mut self, device_id: DeviceID, table_block: BlockNum, index: usize, op: GetFileBlockOp) -> Result<Option<BlockNum>, KernelError> {
        let buf = block::get_buf(device_id, table_block)?;

        let block = {
            let locked_buf = buf.lock();
            let table = unsafe { memory::cast_to_slice(&*locked_buf) };
            table[index]
        };

        if block != 0 {
            Ok(Some(block))
        } else if op == GetFileBlockOp::Lookup {
            Ok(None)
        } else {
            let mut locked_buf = buf.lock_mut();
            let table = unsafe { memory::cast_to_slice_mut(&mut *locked_buf) };
            table[index] = self.get_mount().alloc_block(self.attrs.inode as Ext2InodeNum)?;
            Ok(Some(table[index]))
        }
    }

    fn get_number_of_tiers(&self, linear_block_num: usize) -> Result<usize, KernelError> {
        let entries_per_block = self.get_block_size() / mem::size_of::<Ext2BlockNumber>();
        let remaining_offset = linear_block_num - EXT2_INODE_DIRECT_BLOCKS;

        if linear_block_num < EXT2_INODE_DIRECT_BLOCKS {
            Ok(0)
        } else if remaining_offset < entries_per_block {
            Ok(1)
        } else if remaining_offset < entries_per_block * entries_per_block {
            Ok(2)
        } else if remaining_offset < entries_per_block * entries_per_block * entries_per_block {
            Ok(3)
        } else {
            Err(KernelError::FileSizeTooLarge)
        }
    }

    pub(super) fn free_all_blocks(&mut self) -> Result<(), KernelError> {
        self.dirty = true;

        for index in 0..EXT2_INODE_DIRECT_BLOCKS {
            if self.blocks[index] != 0 {
                self.get_mount().superblock.free_block(self.blocks[index])?;
                self.blocks[index] = 0;
            }
        }

        let device_id = self.get_device_id();
        self.free_blocks_in_tier(device_id, 1, self.blocks[EXT2_INODE_DIRECT_BLOCKS])?;
        self.free_blocks_in_tier(device_id, 2, self.blocks[EXT2_INODE_DIRECT_BLOCKS + 1])?;
        self.free_blocks_in_tier(device_id, 3, self.blocks[EXT2_INODE_DIRECT_BLOCKS + 2])?;

        self.attrs.size = 0;
        Ok(())
    }

    fn free_blocks_in_tier(&mut self, device_id: DeviceID, tier: usize, table_block: BlockNum) -> Result<(), KernelError> {
        if table_block == 0 {
            return Ok(());
        }

        let buf = block::get_buf(device_id, table_block)?;
        let mut locked_buf = buf.lock_mut();
        let table_data = unsafe { memory::cast_to_slice_mut(&mut *locked_buf) };

        for index in 0..table_data.len() {
            if table_data[index] != 0 {
                if tier > 1 {
                    self.free_blocks_in_tier(device_id, tier - 1, table_data[index])?;
                }
                self.get_mount().superblock.free_block(table_data[index])?;
                table_data[index] = 0;
            }
        }
        Ok(())
    }
}


impl Ext2Mount {
    pub(super) fn alloc_block(&mut self, near_inode: Ext2InodeNum) -> Result<Ext2BlockNumber, KernelError> {
        self.superblock.alloc_block(near_inode)
    }
}

