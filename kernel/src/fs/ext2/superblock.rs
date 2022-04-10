
use core::slice;

use alloc::vec::Vec;

use ruxpin_api::types::DeviceID;

use crate::block;
use crate::printkln;
use crate::block::BlockNum;
use crate::misc::ceiling_div;
use crate::errors::KernelError;
use crate::misc::byteorder::{leu16, leu32};

use super::Ext2InodeNum;


const EXT2_INCOMPAT_FILE_TYPE_IN_DIRS: u32       = 0x00002;
const EXT2_INCOMPAT_FS_NEEDS_RECOVERY: u32       = 0x00004;
const EXT2_INCOMPAT_FLEX_BLOCK_GROUP: u32        = 0x00200;
const EXT2_INCOMPAT_SUPPORTED: u32               = EXT2_INCOMPAT_FILE_TYPE_IN_DIRS;

#[repr(C)]
struct Ext2SuperBlockOnDisk {
    total_inodes: leu32,
    total_blocks: leu32,
    reserved_su_blocks: leu32,
    total_unalloc_blocks: leu32,
    total_unalloc_inodes: leu32,
    superblock_block: leu32,
    log_block_size: leu32,
    log_fragment_size: leu32,

    blocks_per_group: leu32,
    fragments_per_block: leu32,
    inodes_per_group: leu32,

    last_mount_time: leu32,
    last_write_time: leu32,

    mounts_since_check: leu16,
    mounts_before_check: leu16,
    magic: leu16,
    state: leu16,

    errors: leu16,
    minor_version: leu16,
    last_check: leu32,
    check_interval: leu32,
    creator_os: leu32,
    major_version: leu32,

    reserved_uid: leu16,
    reserved_gid: leu16,

    extended: Ext2ExtendedSuperBlockOnDisk,
}

#[repr(C)]
#[derive(Debug)]
struct Ext2ExtendedSuperBlockOnDisk {
    first_non_reserved_inode: leu32,
    inode_size: leu16,
    blockgroup_of_super: leu16,
    compat_features: leu32,
    incompat_features: leu32,
    ro_compat_features: leu32,
}

#[repr(C)]
struct Ext2GroupDescriptorOnDisk {
    block_bitmap: leu32,
    inode_bitmap: leu32,
    inode_table: leu32,
    free_block_count: leu16,
    free_inode_count: leu16,
    used_dirs_count: leu16,
    _padding: leu16,
    _reserved: [leu32; 3],
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

        if u16::from(data.magic) != 0xEF53 {
            printkln!("ext2: invalid superblock magic number {:x}", u16::from(data.magic));
            return Err(KernelError::InvalidSuperblock);
        }

        let block_size = 1024 << u32::from(data.log_block_size) as usize;
        let fragment_size = 1024 << u32::from(data.log_fragment_size) as usize;

        if block_size != fragment_size {
            printkln!("ext2: block size and fragment size don't match");
            return Err(KernelError::InvalidSuperblock);
        }

        let incompat_features = u32::from(data.extended.incompat_features);
        if (incompat_features & !EXT2_INCOMPAT_SUPPORTED) != 0 {
            printkln!("ext2: this filesystem has incompatible features than aren't supported: {:x}", incompat_features & !EXT2_INCOMPAT_SUPPORTED);
            return Err(KernelError::IncompatibleFeatures);
        }

        let inode_size = if u32::from(data.major_version) >= 1 { u16::from(data.extended.inode_size) as usize } else { 128 };
        let total_blocks = data.total_blocks.into();
        let blocks_per_group = data.blocks_per_group.into();
        let total_block_groups = ceiling_div(total_blocks as usize, blocks_per_group as usize);

        let superblock = Self {
            total_inodes: data.total_inodes.into(),
            total_blocks,
            reserved_su_blocks: data.reserved_su_blocks.into(),
            total_unalloc_blocks: data.total_unalloc_blocks.into(),
            total_unalloc_inodes: data.total_unalloc_inodes.into(),
            superblock_block: data.superblock_block.into(),
            block_size,
            fragment_size,

            blocks_per_group,
            fragments_per_block: data.fragments_per_block.into(),
            inodes_per_group: data.inodes_per_group.into(),

            last_mount_time: data.last_mount_time.into(),
            last_write_time: data.last_write_time.into(),

            mounts_since_check: data.mounts_since_check.into(),
            mounts_before_check: data.mounts_before_check.into(),
            magic: data.magic.into(),
            state: data.state.into(),

            inode_size,
            inodes_per_block: block_size / inode_size,

            total_block_groups,
            groups: Vec::with_capacity(total_block_groups),
        };

        Ok(superblock)
    }

    pub(super) fn get_block_size(&self) -> usize {
        self.block_size
    }

    pub(super) fn get_inode_entry_location(&self, inode_num: Ext2InodeNum) -> Result<(BlockNum, usize), KernelError> {
        let(group, group_inode) = self.get_inode_group_and_offset(inode_num)?;

        let block = self.groups[group].inode_table + (group_inode / self.inodes_per_block as u32);
        Ok((block, (group_inode as usize % self.inodes_per_block) * self.inode_size))
    }

    pub(super) fn get_inode_bitmap_location(&self, inode_num: Ext2InodeNum) -> Result<(BlockNum, usize), KernelError> {
        let(group, group_inode) = self.get_inode_group_and_offset(inode_num)?;

        let bitmap = self.groups[group].inode_bitmap;
        Ok((bitmap, group_inode as usize))
    }

    fn get_inode_group_and_offset(&self, inode_num: Ext2InodeNum) -> Result<(usize, Ext2InodeNum), KernelError> {
        let group = ((inode_num - 1) / self.inodes_per_group) as usize;
        let group_inode = (inode_num - 1) % self.inodes_per_group;

        if inode_num >= self.total_inodes || group >= self.groups.len() {
            return Err(KernelError::InvalidInode);
        }
        Ok((group, group_inode))
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
                block_bitmap: data[i].block_bitmap.into(),
                inode_bitmap: data[i].inode_bitmap.into(),
                inode_table: data[i].inode_table.into(),
                free_block_count: data[i].free_block_count.into(),
                free_inode_count: data[i].free_inode_count.into(),
                used_dirs_count: data[i].used_dirs_count.into(),
            };

            groups.push(group);
        }

        Ok(())
    }
}

