
use alloc::vec::Vec;

use ruxpin_api::types::DeviceID;

use ruxpin_kernel::block;
use ruxpin_kernel::printkln;
use ruxpin_kernel::block::BlockNum;
use ruxpin_kernel::misc::ceiling_div;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::misc::byteorder::{leu16, leu32};
use ruxpin_kernel::misc::memory::{cast_to_slice, cast_to_slice_mut};

use super::Ext2InodeNum;
use super::Ext2BlockNumber;


#[allow(dead_code)] const EXT2_INCOMPAT_FILE_TYPE_IN_DIRS: u32       = 0x00002;
#[allow(dead_code)] const EXT2_INCOMPAT_FS_NEEDS_RECOVERY: u32       = 0x00004;
#[allow(dead_code)] const EXT2_INCOMPAT_FLEX_BLOCK_GROUP: u32        = 0x00200;

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

    blocks_per_group: usize,
    inodes_per_group: usize,

    magic: u16,
    state: u16,

    inode_size: usize,
    pub(super) inodes_per_block: usize,

    device_id: DeviceID,
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

        // Change the blocksize to the expected disk blocksize (will clear cache)
        block::set_buf_size(device_id, superblock.block_size)?;

        Ext2BlockGroup::load_all(device_id, superblock.superblock_block + 1, &mut superblock.groups, superblock.total_block_groups)?;
        Ok(superblock)
    }

    pub fn store(&self) -> Result<(), KernelError> {
        self.update_superblock()?;
        Ext2BlockGroup::store_all(self.device_id, self.superblock_block + 1, &self.groups)?;
        Ok(())
    }

    fn read_superblock(device_id: DeviceID) -> Result<Ext2SuperBlock, KernelError> {
        block::set_buf_size(device_id, 1024)?;
        let buf = block::get_buf(device_id, 1)?;

        let data = unsafe {
            &*(buf.lock().as_ptr() as *mut Ext2SuperBlockOnDisk)
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
        let blocks_per_group = u32::from(data.blocks_per_group) as usize;
        let total_block_groups = ceiling_div(total_blocks as usize, blocks_per_group);

        let superblock = Self {
            total_inodes: data.total_inodes.into(),
            total_blocks,
            reserved_su_blocks: data.reserved_su_blocks.into(),
            total_unalloc_blocks: data.total_unalloc_blocks.into(),
            total_unalloc_inodes: data.total_unalloc_inodes.into(),
            superblock_block: data.superblock_block.into(),
            block_size,

            blocks_per_group,
            inodes_per_group: u32::from(data.inodes_per_group) as usize,

            magic: data.magic.into(),
            state: data.state.into(),

            device_id,
            inode_size,
            inodes_per_block: block_size / inode_size,

            total_block_groups,
            groups: Vec::with_capacity(total_block_groups),
        };

        printkln!("ext2: magic number {:x}, block size {}", superblock.magic, superblock.block_size);
        printkln!("ext2: total blocks {}, total inodes {}, unallocated blocks: {}, unallocated inodes: {}", superblock.total_blocks, superblock.total_inodes, superblock.total_unalloc_blocks, superblock.total_unalloc_inodes);
        printkln!("ext2: features compat: {:x}, ro: {:x}, incompat: {:x}", u32::from(data.extended.compat_features), u32::from(data.extended.ro_compat_features), u32::from(data.extended.incompat_features));

        Ok(superblock)
    }

    pub(super) fn update_superblock(&self) -> Result<(), KernelError> {
        let block_size = block::get_buf_size(self.device_id)?;
        let superblock_block = if block_size <= 1024 { 1 } else { 0 };
        let buf = block::get_buf(self.device_id, superblock_block)?;

        let data = unsafe {
            &mut *(buf.lock_mut().as_mut_ptr() as *mut Ext2SuperBlockOnDisk)
        };

        data.total_unalloc_blocks = self.total_unalloc_blocks.into();
        data.total_unalloc_inodes = self.total_unalloc_inodes.into();
        data.state = self.state.into();

        Ok(())
    }

    pub(super) fn get_block_size(&self) -> usize {
        self.block_size
    }
}

impl Ext2BlockGroup {
    pub fn load_all(device_id: DeviceID, block_num: BlockNum, groups: &mut Vec<Ext2BlockGroup>, total_block_groups: usize) -> Result<(), KernelError> {
        let buf = block::get_buf(device_id, block_num)?;
        let locked_buf = &*buf.lock();

        let data: &[Ext2GroupDescriptorOnDisk] = unsafe {
            cast_to_slice(locked_buf)
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

    pub fn store_all(device_id: DeviceID, block_num: BlockNum, groups: &Vec<Ext2BlockGroup>) -> Result<(), KernelError> {
        let buf = block::get_buf(device_id, block_num)?;
        let locked_buf = &mut *buf.lock_mut();

        let data: &mut [Ext2GroupDescriptorOnDisk] = unsafe {
            cast_to_slice_mut(locked_buf)
        };

        for (i, group) in groups.iter().enumerate() {
            data[i].block_bitmap = group.block_bitmap.into();
            data[i].inode_bitmap = group.inode_bitmap.into();
            data[i].inode_table = group.inode_table.into();
            data[i].free_block_count = group.free_block_count.into();
            data[i].free_inode_count = group.free_inode_count.into();
            data[i].used_dirs_count = group.used_dirs_count.into();
        }

        Ok(())
    }
}



/// Inodes ///

impl Ext2SuperBlock {
    pub(super) fn alloc_inode(&mut self, start_from: Ext2InodeNum) -> Result<Ext2InodeNum, KernelError> {
        let starting_group = start_from as usize / self.inodes_per_group;

        let mut group = starting_group;
        loop {
            if self.groups[group].free_inode_count >= 1 {
                let buf = block::get_buf(self.device_id, self.groups[group].inode_bitmap)?;
                let locked_buf = &mut *buf.lock_mut();

                let bit = alloc_bit(locked_buf, self.inodes_per_group).ok_or(KernelError::OutOfDiskSpace)?;
                self.groups[group].free_inode_count -= 1;
                self.total_unalloc_inodes -= 1;

                return Ok(((group * self.inodes_per_group) + bit + 1) as Ext2InodeNum);
            }

            group += 1;
            if group == starting_group {
                break;
            } else if group >= self.total_block_groups {
                group = 0;
            }
        }

        Err(KernelError::OutOfDiskSpace)
    }

    pub(super) fn free_inode(&mut self, inode_num: Ext2InodeNum) -> Result<(), KernelError> {
        let (group, group_inode) = self.get_inode_group_and_offset(inode_num)?;
        let buf = block::get_buf(self.device_id, self.groups[group].inode_bitmap)?;
        let locked_buf = &mut *buf.lock_mut();

        free_bit(locked_buf, group_inode as usize);
        self.groups[group].free_inode_count += 1;
        self.total_unalloc_inodes += 1;
        Ok(())
    }

    pub(super) fn check_inode_is_allocated(&self, inode_num: Ext2InodeNum) -> Result<(), KernelError> {
        let (group, group_inode) = self.get_inode_group_and_offset(inode_num)?;
        let buf = block::get_buf(self.device_id, self.groups[group].inode_bitmap)?;
        let locked_buf = &*buf.lock();

        if locked_buf[group_inode as usize / 8] & (1 << (group_inode % 8)) != 0 {
            Ok(())
        } else {
            Err(KernelError::InvalidInode)
        }
    }

    pub(super) fn get_inode_entry_location(&self, inode_num: Ext2InodeNum) -> Result<(BlockNum, usize), KernelError> {
        let (group, group_inode) = self.get_inode_group_and_offset(inode_num)?;

        let block = self.groups[group].inode_table + (group_inode / self.inodes_per_block as u32);
        Ok((block, (group_inode as usize % self.inodes_per_block) * self.inode_size))
    }

    fn get_inode_group_and_offset(&self, inode_num: Ext2InodeNum) -> Result<(usize, Ext2InodeNum), KernelError> {
        let group = (inode_num as usize - 1) / self.inodes_per_group;
        let group_inode = (inode_num as usize - 1) % self.inodes_per_group;

        if inode_num >= self.total_inodes || group >= self.groups.len() {
            return Err(KernelError::InvalidInode);
        }
        Ok((group, group_inode as Ext2InodeNum))
    }
}

/// Blocks ///

impl Ext2SuperBlock {
    pub(super) fn alloc_block(&mut self, start_from_inode: Ext2InodeNum) -> Result<Ext2BlockNumber, KernelError> {
        let starting_group = start_from_inode as usize / self.inodes_per_group;

        let mut group = starting_group;
        loop {
            if self.groups[group].free_block_count >= 1 {
                let buf = block::get_buf(self.device_id, self.groups[group].block_bitmap)?;
                let locked_buf = &mut *buf.lock_mut();

                let bit = alloc_bit(locked_buf, self.blocks_per_group).ok_or(KernelError::OutOfDiskSpace)?;
                self.groups[group].free_block_count -= 1;
                self.total_unalloc_blocks -= 1;

                let block_num = ((group * self.blocks_per_group) + bit) as Ext2BlockNumber;
                self.zero_block(block_num)?;

                crate::printkln!("ext2: allocating block {} in group {}", block_num, group);
                return Ok(block_num);
            }

            group += 1;
            if group == starting_group {
                break;
            } else if group >= self.total_block_groups {
                group = 0;
            }
        }

        Err(KernelError::OutOfDiskSpace)
    }

    pub(super) fn free_block(&mut self, block_num: BlockNum) -> Result<(), KernelError> {
        let (group, group_block) = self.get_block_group_and_offset(block_num)?;
        let buf = block::get_buf(self.device_id, self.groups[group].block_bitmap)?;
        let locked_buf = &mut *buf.lock_mut();

        free_bit(locked_buf, group_block as usize);
        self.groups[group].free_block_count += 1;
        self.total_unalloc_blocks += 1;
        Ok(())
    }

    fn get_block_group_and_offset(&self, block_num: Ext2BlockNumber) -> Result<(usize, usize), KernelError> {
        let group = block_num as usize / self.blocks_per_group;
        let group_block = block_num as usize % self.blocks_per_group;

        if block_num >= self.total_blocks || group >= self.groups.len() {
            return Err(KernelError::InvalidInode);
        }
        Ok((group, group_block))
    }

    fn zero_block(&self, block_num: Ext2BlockNumber) -> Result<(), KernelError> {
        let buf = block::get_buf(self.device_id, block_num as BlockNum)?;
        let locked_buf = &mut *buf.lock_mut();
        for i in 0..locked_buf.len() {
            locked_buf[i] = 0;
        }
        Ok(())
    }
}

pub fn alloc_bit(table: &mut [u8], table_size: usize) -> Option<usize> {
    let mut i = 0;

    while i < table_size {
        if table[i] != 0xff {
            let mut bit = 0;
            while bit < 7 && (table[i] & (0x01 << bit)) != 0 {
                bit += 1;
            }
            table[i] |= 0x01 << bit;
            return Some((i * 8) + bit);
        }

        i += 1;
    }

    None
}

fn free_bit(table: &mut [u8], bitnum: usize) {
    let i = bitnum >> 3;
    let bit = bitnum & 0x7;
    table[i] &= !(0x01 << bit);
}

