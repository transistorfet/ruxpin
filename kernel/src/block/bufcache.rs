
use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::misc::cache::{Cache, CacheArc};

use super::BlockOperations;

pub type BlockNum = u32;

pub struct Entry<const SIZE: usize> {
    num: BlockNum,
    block: Spinlock<[u8; SIZE]>,
}

pub struct BufCache {
    block_size: usize,
    cache: Cache<Entry<1024>>,
}

impl BufCache {
    pub fn new(block_size: usize) -> Self {
        Self {
            block_size,
            cache: Cache::new(20),
        }
    }

    pub fn read(&mut self, dev: &mut Box<dyn BlockOperations>, buffer: &mut [u8], offset: usize) -> Result<usize, KernelError> {
        let mut buffer_start = 0;
        let mut buffer_remain = buffer.len();
        let mut block_num = (offset / self.block_size) as BlockNum;
        let mut block_start = offset % self.block_size;
        while buffer_remain > 0 {
            let block_end = if buffer_remain > self.block_size - block_start { self.block_size } else { buffer_remain };
            let entry = self.get_block(dev, block_num)?;
            buffer[buffer_start..].copy_from_slice(&entry.block.lock()[block_start..block_end]);

            buffer_remain = buffer_remain.saturating_sub(self.block_size - block_start);
            buffer_start += self.block_size - block_start;
            block_num += 1;
            block_start = 0;
        }
        Ok(0)
    }

    pub fn write(&mut self, buffer: &[u8], offset: usize) -> Result<usize, KernelError> {

        Err(KernelError::OperationNotPermitted)
    }

    pub fn get_block(&mut self, dev: &mut Box<dyn BlockOperations>, num: BlockNum) -> Result<CacheArc<Entry<1024>>, KernelError> {
        self.cache.get(|entry| entry.num == num, || {
            let entry = Entry::new(num);
            dev.read(&mut *entry.block.lock(), num as usize * self.block_size)?;
            Ok(entry)
        })
    }
}

impl<const SIZE: usize> Entry<SIZE> {
    pub fn new(num: BlockNum) -> Self {
        Self {
            num,
            block: Spinlock::new([0; SIZE]),
        }
    }
}

