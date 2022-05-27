
use core::str;

use ruxpin_types::{FileAccess, DirEntry};

use ruxpin_kernel::block;
use ruxpin_kernel::block::Buf;
use ruxpin_kernel::misc::align_up;
use ruxpin_kernel::block::BlockNum;
use ruxpin_kernel::errors::KernelError;
use ruxpin_kernel::misc::memory::{cast_to_ref, cast_to_ref_mut};
use ruxpin_kernel::misc::byteorder::{leu16, leu32};

use super::Ext2InodeNum;
use super::inodes::Ext2Vnode;
use super::blocks::GetFileBlockOp;


#[allow(dead_code)] const EXT2_FT_UNKNOWN: u8	= 0;
#[allow(dead_code)] const EXT2_FT_REG_FILE: u8	= 1;
#[allow(dead_code)] const EXT2_FT_DIR: u8	= 2;
#[allow(dead_code)] const EXT2_FT_CHRDEV: u8	= 3;
#[allow(dead_code)] const EXT2_FT_BLKDEV: u8	= 4;
#[allow(dead_code)] const EXT2_FT_FIFO: u8	= 5;
#[allow(dead_code)] const EXT2_FT_SOCK: u8	= 6;
#[allow(dead_code)] const EXT2_FT_SYMLINK: u8   = 7;

#[repr(C)]
struct Ext2DirEntryHeader {
    inode: leu32,
    entry_len: leu16,
    name_len: u8,
    file_type: u8,
}

impl Ext2Vnode {
    pub(super) fn read_next_dirent_from_vnode(&mut self, dirent: &mut DirEntry, position: usize) -> Result<Option<usize>, KernelError> {
        let block_size = self.get_block_size();
        let device_id = self.get_device_id();

        // Find the block number that corresponds to the current position and return if we're at the end of the directory
        let znum = position / block_size;
        if znum * block_size > self.attrs.size {
            return Err(KernelError::FileNotFound);
        }

        let block_num = match self.get_file_block_num(znum, GetFileBlockOp::Lookup)? {
            None => { return Ok(None); },
            Some(num) => num,
        };
        let buf = block::get_buf(device_id, block_num)?;
        let locked_buf = &*buf.lock();

        // Get the directory entry's position in the block
        let offset = position % block_size;
        let entry_on_disk: &Ext2DirEntryHeader = unsafe {
            cast_to_ref(&locked_buf[offset..])
        };

        let entry_len = u16::from(entry_on_disk.entry_len) as usize;
        let name_len = entry_on_disk.name_len as usize;

        // Copy data into the directory entry pointer we were given
        dirent.inode = u32::from(entry_on_disk.inode);
        dirent.copy_into(&locked_buf[(offset + 8)..(offset + 8 + name_len)]);

        // return the length of the entry (which added to the position we were given gives the next entry)
        Ok(Some(entry_len))
    }

    pub(super) fn add_directory_to_vnode(&mut self, access: FileAccess, filename: &str, inode: Ext2InodeNum) -> Result<(), KernelError> {
        let device_id = self.get_device_id();
        let name_len = filename.len();
        let min_entry_len = align_up(name_len + 8, 4);

        let (block_num, mut offset) = self.find_directory_space(min_entry_len)?;
        let buf = block::get_buf(device_id, block_num)?;
        let locked_buf = &mut *buf.lock_mut();

        let mut entry_on_disk: &mut Ext2DirEntryHeader = unsafe {
            cast_to_ref_mut(&mut locked_buf[offset..])
        };

        // Split the entry if needed
        if u32::from(entry_on_disk.inode) != 0 {
            let entry_min_len = align_up(entry_on_disk.name_len as usize + 8, 4);
            let entry_len = u16::from(entry_on_disk.entry_len);
            entry_on_disk.entry_len = (entry_min_len as u16).into();

            offset += entry_min_len;
            entry_on_disk = unsafe {
                cast_to_ref_mut(&mut locked_buf[offset..])
            };
            entry_on_disk.entry_len = (entry_len - entry_min_len as u16).into();
        }

        entry_on_disk.inode = inode.into();
        entry_on_disk.name_len = name_len as u8;
        entry_on_disk.file_type = to_file_type(access);
        offset += 8;
        locked_buf[offset..offset + name_len].copy_from_slice(filename.as_bytes());
        Ok(())
    }

    fn find_directory_space(&mut self, min_entry_len: usize) -> Result<(BlockNum, usize), KernelError> {
        let block_size = self.get_block_size();
        let device_id = self.get_device_id();

        let mut znum = 0;
        while znum * block_size <= self.attrs.size {
            let block_num = match self.get_file_block_num(znum, GetFileBlockOp::Lookup)? {
                None => { break; },
                Some(num) => num,
            };
            let buf = block::get_buf(device_id, block_num)?;
            let locked_buf = &*buf.lock();

            let mut offset = 0;
            while offset < block_size {
                let entry_on_disk: &Ext2DirEntryHeader = unsafe {
                    cast_to_ref(&locked_buf[offset..])
                };

                let entry_len = u16::from(entry_on_disk.entry_len) as usize;
                let entry_min_len = align_up(entry_on_disk.name_len as usize + 8, 4);

                if entry_len > entry_min_len + min_entry_len {
                    return Ok((block_num, offset));
                }

                offset += entry_len;
            }

            znum += 1;
        }

        // No existing entries can be split, so we add a new block
        let block_num = self.get_file_block_num(znum, GetFileBlockOp::Allocate)?.unwrap();
        self.attrs.size += block_size;
        let buf = block::get_buf(device_id, block_num)?;
        let locked_buf = &mut *buf.lock_mut();

        let entry_on_disk: &mut Ext2DirEntryHeader = unsafe {
            cast_to_ref_mut(locked_buf)
        };

        // Initialize the entry before returning, in case it contains non-zero data
        entry_on_disk.inode = 0.into();
        entry_on_disk.entry_len = (block_size as u16).into();
        Ok((block_num, 0))
    }

    pub(super) fn remove_directory_entry(&mut self, filename: &str) -> Result<Ext2InodeNum, KernelError> {
        let block_size = self.get_block_size();
        let device_id = self.get_device_id();

        let mut znum = 0;
        while znum * block_size <= self.attrs.size {
            let block_num = match self.get_file_block_num(znum, GetFileBlockOp::Lookup)? {
                None => { break; },
                Some(num) => num,
            };
            let buf = block::get_buf(device_id, block_num)?;

            let mut position = 0;
            let mut previous_position = None;
            while position < block_size {
                let offset = position % block_size;
                let (is_equal, entry_len, inode) = compare_filename_from_buf(&buf, offset, filename);

                if is_equal {
                    if previous_position.is_none() {
                        // TODO this should actually delete the block
                        return Err(KernelError::InvalidInode);
                    } else {
                        let locked_buf = &mut *buf.lock_mut();
                        let mut previous_entry_on_disk: &mut Ext2DirEntryHeader = unsafe {
                            cast_to_ref_mut(&mut locked_buf[previous_position.unwrap()..])
                        };

                        previous_entry_on_disk.entry_len = (u16::from(previous_entry_on_disk.entry_len) + entry_len as u16).into();
                        return Ok(inode);
                    }
                }

                previous_position = Some(position);
                position += entry_len;
            }

            znum += 1;
        }

        Err(KernelError::FileNotFound)
    }
}

fn compare_filename_from_buf(buf: &Buf, offset: usize, filename: &str) -> (bool, usize, Ext2InodeNum) {
    let locked_buf = &*buf.lock();

    let entry_on_disk: &Ext2DirEntryHeader = unsafe {
        cast_to_ref(&locked_buf[offset..])
    };

    let inode = u32::from(entry_on_disk.inode);
    let entry_len = u16::from(entry_on_disk.entry_len) as usize;

    let is_equal = filename.as_bytes() == &locked_buf[(offset + 8)..(offset + 8 + entry_on_disk.name_len as usize)];
    (is_equal, entry_len, inode)
}

fn to_file_type(access: FileAccess) -> u8 {
    match access.file_type() {
        FileAccess::Regular             => EXT2_FT_REG_FILE,
        FileAccess::Directory           => EXT2_FT_DIR,
        FileAccess::CharDevice          => EXT2_FT_CHRDEV,
        FileAccess::BlockDevice         => EXT2_FT_BLKDEV,
        FileAccess::Fifo                => EXT2_FT_FIFO,
        FileAccess::Socket              => EXT2_FT_SOCK,
        FileAccess::SymbolicLink        => EXT2_FT_SYMLINK,
        _                               => EXT2_FT_UNKNOWN,
    }
}

