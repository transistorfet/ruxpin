
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::vec;
use alloc::boxed::Box;

use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::misc::cache::{Cache, CacheArc};

use super::BlockOperations;

pub type BlockNum = u32;

pub struct Buf {
    dirty: AtomicBool,
    pub block: Spinlock<Box<[u8]>>,
}

pub struct BufCache {
    block_size: usize,
    cache: Cache<BlockNum, Buf>,
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

    pub fn set_block_size(&mut self, block_size: usize) -> Result<(), KernelError> {
        self.cache.clear().map_err(|_| KernelError::OperationNotPermitted)?;
        self.block_size = block_size;
        Ok(())
    }

    pub fn get_block(&mut self, dev: &mut Box<dyn BlockOperations>, num: BlockNum) -> Result<CacheArc<BlockNum, Buf>, KernelError> {
        self.cache.get(num, || {
            let entry = Buf::new(self.block_size);
            let nbytes = dev.read(&mut *entry.block.lock(), (num as usize * self.block_size) as u64)?;
            if nbytes != self.block_size {
                return Err(KernelError::IOError);
            }

            // TODO this is for debugging
            //crate::printkln!("buf {}", num);
            //unsafe { crate::printk::printk_dump((&**entry.block.lock()).as_ptr(), self.block_size); }

            Ok(entry)
        })
    }

    pub fn write_block(&mut self, dev: &mut Box<dyn BlockOperations>, num: BlockNum) -> Result<(), KernelError> {
        let buf = self.get_block(dev, num)?;
        let nbytes = dev.write(&*buf.block.lock(), (num as usize * self.block_size) as u64)?;
        if nbytes != self.block_size {
            return Err(KernelError::IOError);
        }
        buf.dirty.store(false, Ordering::Release);
        Ok(())
    }
}

impl Buf {
    pub fn new(block_size: usize) -> Self {
        Self {
            dirty: AtomicBool::new(false),
            block: Spinlock::new(vec![0; block_size].into_boxed_slice()),
        }
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        if self.dirty.load(Ordering::Acquire) {
            panic!("bufcache: buf was not written back after use");
        }
    }
}

