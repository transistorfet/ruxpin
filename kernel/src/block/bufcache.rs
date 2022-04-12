
use core::ops::Deref;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::vec;
use alloc::boxed::Box;

use ruxpin_api::types::DeviceID;

use crate::block;
use crate::errors::KernelError;
use crate::misc::cache::{Cache, CacheArc};
use crate::sync::{Spinlock, SpinlockGuard};


pub type BlockNum = u32;

pub struct Buf {
    dirty: AtomicBool,
    block: Spinlock<Box<[u8]>>,
}

pub struct BufCache {
    device_id: DeviceID,
    block_size: usize,
    cache: Cache<BlockNum, Buf>,
}

impl BufCache {
    pub fn new(device_id: DeviceID, block_size: usize) -> Self {
        Self {
            device_id,
            block_size,
            cache: Cache::new(20),
        }
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn set_block_size(&mut self, block_size: usize) -> Result<(), KernelError> {
        self.cache.clear(|key, buf| {
            store_buf(self.device_id, key, self.block_size, buf)
        })?;
        self.block_size = block_size;
        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), KernelError> {
        self.cache.commit(|key, buf| {
            store_buf(self.device_id, key, self.block_size, buf)
        })
    }

    pub fn get_block(&mut self, block_num: BlockNum) -> Result<CacheArc<BlockNum, Buf>, KernelError> {
        self.cache.get(block_num, || {
            load_buf(self.device_id, block_num, self.block_size)
        }, |key, buf| {
            store_buf(self.device_id, key, self.block_size, buf)
        })
    }

    pub fn write_block(&mut self, block_num: BlockNum) -> Result<(), KernelError> {
        let buf = self.get_block(block_num)?;
        store_buf(self.device_id, block_num, self.block_size, &*buf)
    }
}

fn load_buf(device_id: DeviceID, block_num: BlockNum, block_size: usize) -> Result<Buf, KernelError> {
    let entry = Buf::new(block_size);
    let nbytes = block::raw_read(device_id, &mut *entry.block.lock(), (block_num as usize * block_size) as u64)?;
    if nbytes != block_size {
        return Err(KernelError::IOError);
    }

    // TODO this is for debugging
    //crate::printkln!("buf {}", num);
    //unsafe { crate::printk::printk_dump((&**entry.block.lock()).as_ptr(), self.block_size); }

    Ok(entry)
}

fn store_buf(device_id: DeviceID, block_num: BlockNum, block_size: usize, buf: &Buf) -> Result<(), KernelError> {
    if buf.dirty.load(Ordering::Acquire) {
        let nbytes = block::raw_write(device_id, &*buf.block.lock(), (block_num as usize * block_size) as u64)?;
        if nbytes != block_size {
            return Err(KernelError::IOError);
        }
        buf.dirty.store(false, Ordering::Release);
    }
    Ok(())
}


impl Buf {
    pub fn new(block_size: usize) -> Self {
        Self {
            dirty: AtomicBool::new(false),
            block: Spinlock::new(vec![0; block_size].into_boxed_slice()),
        }
    }

    pub fn lock(&self) -> BufGuard<'_> {
        BufGuard {
            guard: self.block.lock(),
        }
    }

    pub fn lock_mut(&self) -> SpinlockGuard<'_, Box<[u8]>> {
        self.dirty.store(true, Ordering::Release);
        self.block.lock()
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        if self.dirty.load(Ordering::Acquire) {
            panic!("bufcache: buf was not written back after use");
        }
    }
}


pub struct BufGuard<'a> {
    guard: SpinlockGuard<'a, Box<[u8]>>,
}

impl<'a> Deref for BufGuard<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

