
use alloc::vec;
use alloc::boxed::Box;

use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::misc::cache::{Cache, CacheArc};

use super::BlockOperations;

pub type BlockNum = u32;

pub struct Buf {
    pub num: BlockNum,
    pub block: Spinlock<Box<[u8]>>,
}

pub struct BufCache {
    block_size: usize,
    cache: Cache<Buf>,
}

impl BufCache {
    pub fn new(block_size: usize) -> Self {
        Self {
            block_size,
            cache: Cache::new(20),
        }
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn read(&mut self, dev: &mut Box<dyn BlockOperations>, buffer: &mut [u8], offset: u64) -> Result<usize, KernelError> {
        let block_size = self.block_size as u64;

        let mut buffer_start = 0;
        let mut buffer_remain = buffer.len() as u64;
        let mut block_num = (offset / block_size) as BlockNum;
        let mut block_start = offset % block_size;
        while buffer_remain > 0 {
            let block_end = if buffer_remain > block_size - block_start { block_size } else { buffer_remain };
            let entry = self.get_block(dev, block_num)?;
            buffer[buffer_start..].copy_from_slice(&entry.block.lock()[block_start as usize..block_end as usize]);

            buffer_remain = buffer_remain.saturating_sub(block_size - block_start);
            buffer_start += (block_size - block_start) as usize;
            block_num += 1;
            block_start = 0;
        }
        Ok(0)
    }

    pub fn write(&mut self, buffer: &[u8], offset: u64) -> Result<usize, KernelError> {

        Err(KernelError::OperationNotPermitted)
    }

    pub fn get_block(&mut self, dev: &mut Box<dyn BlockOperations>, num: BlockNum) -> Result<CacheArc<Buf>, KernelError> {
        self.cache.get(|entry| entry.num == num, || {
            let entry = Buf::new(num, self.block_size);
            dev.read(&mut *entry.block.lock(), (num as usize * self.block_size) as u64)?;
            Ok(entry)
        })
    }
}

impl Buf {
    pub fn new(num: BlockNum, block_size: usize) -> Self {
        Self {
            num,
            block: Spinlock::new(vec![0; 1024].into_boxed_slice()),
        }
    }
}

