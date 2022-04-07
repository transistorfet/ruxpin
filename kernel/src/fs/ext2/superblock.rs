
use core::slice;

use alloc::vec::Vec;

use ruxpin_api::types::DeviceID;

use crate::block;
use crate::printkln;
use crate::block::BlockNum;
use crate::misc::ceiling_div;
use crate::errors::KernelError;

use super::Ext2InodeNum;


#[repr(C)]
struct Ext2SuperBlockOnDisk {
    total_inodes: u32,
    total_blocks: u32,
    reserved_su_blocks: u32,
    total_unalloc_blocks: u32,
    total_unalloc_inodes: u32,
    superblock_block: u32,
    log_block_size: u32,
    log_fragment_size: u32,

    blocks_per_group: u32,
    fragments_per_block: u32,
    inodes_per_group: u32,

    last_mount_time: u32,
    last_write_time: u32,

    mounts_since_check: u16,
    mounts_before_check: u16,
    magic: u16,
    state: u16,

    errors: u16,
    minor_version: u16,
    last_check: u32,
    check_interval: u32,
    creator_os: u32,
    major_version: u32,

    reserved_uid: u16,
    reserved_gid: u16,

    extended: Ext2ExtendedSuperBlockOnDisk,
}

#[repr(C)]
struct Ext2ExtendedSuperBlockOnDisk {
    first_reserved_inode: u32,
    inode_size: u16,
    blockgroup_of_super: u32,
    optional_features: u32,
}

#[repr(C)]
struct Ext2GroupDescriptorOnDisk {
    block_bitmap: u32,
    inode_bitmap: u32,
    inode_table: u32,
    free_block_count: u16,
    free_inode_count: u16,
    used_dirs_count: u16,
    _padding: u16,
    _reserved: [u32; 3],
}


#[allow(dead_code)]
#[derive(Debug)]
pub struct Ext2SuperBlock {
    total_inodes: u32,
    total_blocks: u32,
    reserved_su_blocks: u32,
    total_unalloc_blocks: u32,
    total_unalloc_inodes: u32,
    superblock_block: u32,
    block_size: usize,
    fragment_size: usize,

    blocks_per_group: u32,
    fragments_per_block: u32,
    inodes_per_group: u32,

    last_mount_time: u32,
    last_write_time: u32,

    mounts_since_check: u16,
    mounts_before_check: u16,
    magic: u16,
    state: u16,

    inode_size: usize,
    pub(super) inodes_per_block: usize,

    total_block_groups: usize,
    groups: Vec<Ext2BlockGroup>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct Ext2BlockGroup {
    block_bitmap: u32,
    inode_bitmap: u32,
    inode_table: u32,
    free_block_count: u16,
    free_inode_count: u16,
    used_dirs_count: u16,
}

impl Ext2SuperBlock {
    pub fn load(device_id: DeviceID) -> Result<Ext2SuperBlock, KernelError> {
        let mut superblock = Ext2SuperBlock::read_superblock(device_id)?;
        block::set_buf_size(device_id, superblock.block_size)?;

        Ext2BlockGroup::read_into(device_id, superblock.superblock_block + 1, &mut superblock.groups, superblock.total_block_groups)?;

        Ok(superblock)
    }

    fn read_superblock(device_id: DeviceID) -> Result<Ext2SuperBlock, KernelError> {
        block::set_buf_size(device_id, 1024)?;
        let buf = block::get_buf(device_id, 1)?;

        let data = unsafe {
            &*((buf.block.lock()).as_ptr() as *mut Ext2SuperBlockOnDisk)
        };

        if data.magic != 0xEF53 {
            printkln!("ext2: invalid superblock magic number {:x}", data.magic);
            return Err(KernelError::InvalidSuperblock);
        }

        if data.log_block_size != data.log_fragment_size {
            printkln!("ext2: block size and fragment size don't match");
            return Err(KernelError::InvalidSuperblock);
        }

        let block_size = 1024 << data.log_block_size as usize;
        let inode_size = if data.major_version > 1 { data.extended.inode_size as usize } else { 128 };
        let total_block_groups = ceiling_div(data.total_blocks as usize, data.blocks_per_group as usize);

        let superblock = Self {
            total_inodes: data.total_inodes,
            total_blocks: data.total_blocks,
            reserved_su_blocks: data.reserved_su_blocks,
            total_unalloc_blocks: data.total_unalloc_blocks,
            total_unalloc_inodes: data.total_unalloc_inodes,
            superblock_block: data.superblock_block,
            block_size: block_size,
            fragment_size: 1024 << data.log_fragment_size as usize,

            blocks_per_group: data.blocks_per_group,
            fragments_per_block: data.fragments_per_block,
            inodes_per_group: data.inodes_per_group,

            last_mount_time: data.last_mount_time,
            last_write_time: data.last_write_time,

            mounts_since_check: data.mounts_since_check,
            mounts_before_check: data.mounts_before_check,
            magic: data.magic,
            state: data.state,

            inode_size,
            inodes_per_block: block_size / inode_size,

            total_block_groups,
            groups: Vec::with_capacity(total_block_groups),
        };

        Ok(superblock)
    }

    pub(super) fn get_inode_location(&self, inode_num: Ext2InodeNum) -> Result<(BlockNum, usize), KernelError> {
        let group = (inode_num / self.inodes_per_group) as usize;
        let group_inode = inode_num % self.inodes_per_group;

        if group_inode >= self.total_inodes || group >= self.groups.len() {
            return Err(KernelError::InvalidInode);
        }

        let block = (self.blocks_per_group * group as u32) + self.groups[group].inode_table + (group_inode / self.inodes_per_block as u32);

        Ok((block, (group_inode as usize % self.inodes_per_block)))
    }
}


impl Ext2BlockGroup {
    pub fn read_into(device_id: DeviceID, block_num: BlockNum, groups: &mut Vec<Ext2BlockGroup>, total_block_groups: usize) -> Result<(), KernelError> {
        let buf = block::get_buf(device_id, block_num)?;

        let data = unsafe {
            slice::from_raw_parts((buf.block.lock()).as_ptr() as *mut Ext2GroupDescriptorOnDisk, total_block_groups)
        };

        for i in 0..total_block_groups {
            let group = Self {
                block_bitmap: data[i].block_bitmap,
                inode_bitmap: data[i].inode_bitmap,
                inode_table: data[i].inode_table,
                free_block_count: data[i].free_block_count,
                free_inode_count: data[i].free_inode_count,
                used_dirs_count: data[i].used_dirs_count,
            };

            groups.push(group);
        }

        Ok(())
    }
}

