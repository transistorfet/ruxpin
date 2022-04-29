
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::boxed::Box;
 
use ruxpin_api::types::{OpenFlags, DeviceID, DriverID, MinorDeviceID};

use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::misc::cache::CacheArc;

pub mod bufcache;
pub mod partition;

pub use self::bufcache::{BlockNum, Buf, BufCache};


pub trait BlockOperations: Sync + Send {
    fn open(&mut self, mode: OpenFlags) -> Result<(), KernelError>;
    fn close(&mut self) -> Result<(), KernelError>;
    fn read(&mut self, buffer: &mut [u8], offset: u64) -> Result<usize, KernelError>;
    fn write(&mut self, buffer: &[u8], offset: u64) -> Result<usize, KernelError>;
    //int (*ioctl)(devminor_t minor, unsigned int request, void *argp, uid_t uid);
    //int (*poll)(devminor_t minor, int events);
    //offset_t (*seek)(devminor_t minor, offset_t position, int whence, offset_t offset);
}

struct BlockDevice {
    dev: Spinlock<Box<dyn BlockOperations>>,
    cache: Spinlock<BufCache>,
}

type BlockDeviceEntry = Arc<BlockDevice>;

struct BlockDriver {
    prefix: &'static str,
    devices: Vec<BlockDeviceEntry>,
}


static BLOCK_DRIVERS: Spinlock<Vec<BlockDriver>> = Spinlock::new(Vec::new());


pub fn register_block_driver(prefix: &'static str) -> Result<DriverID, KernelError> {
    let driver_id = BLOCK_DRIVERS.lock().len() as DriverID;
    BLOCK_DRIVERS.lock().push(BlockDriver::new(prefix));
    Ok(driver_id)
}

pub fn register_block_device(driver_id: DriverID, dev: Box<dyn BlockOperations>) -> Result<MinorDeviceID, KernelError> {
    let mut drivers_list = BLOCK_DRIVERS.lock();
    let driver = drivers_list.get_mut(driver_id as usize).ok_or(KernelError::NoSuchDevice)?;
    driver.add_device(driver_id, dev)
}

pub fn lookup_device(name: &str) -> Result<DeviceID, KernelError> {
    let drivers_list = BLOCK_DRIVERS.lock();
    for (driver_id, driver) in drivers_list.iter().enumerate() {
        if driver.prefix == &name[..driver.prefix.len()] {
            let subdevice_id = name[driver.prefix.len()..].parse::<MinorDeviceID>().map_err(|_| KernelError::NoSuchDevice)?;
            if (subdevice_id as usize) < driver.devices.len() {
                return Ok(DeviceID(driver_id as DriverID, subdevice_id));
            }
            break;
        }
    }
    Err(KernelError::NoSuchDevice)
}

pub fn open(device_id: DeviceID, mode: OpenFlags) -> Result<(), KernelError> {
    let device = get_device(device_id)?;
    let result = device.dev.lock().open(mode);
    result
}

pub fn close(device_id: DeviceID) -> Result<(), KernelError> {
    let device = get_device(device_id)?;
    let result = device.dev.lock().close();
    result
}

pub fn read(device_id: DeviceID, buffer: &mut [u8], offset: u64) -> Result<usize, KernelError> {
    let device = get_device(device_id)?;
    let result = buffered_read(&mut device.cache.lock(), buffer, offset);
    result
}

pub fn write(device_id: DeviceID, buffer: &[u8], offset: u64) -> Result<usize, KernelError> {
    let device = get_device(device_id)?;
    let result = device.dev.lock().write(buffer, offset);
    result
}


pub fn get_buf<'a>(device_id: DeviceID, block_num: BlockNum) -> Result<CacheArc<BlockNum, Buf>, KernelError> {
    let device = get_device(device_id)?;
    let buf = device.cache.lock().get_block(block_num)?;
    Ok(buf)
}

pub fn commit_buf(device_id: DeviceID, block_num: BlockNum) -> Result<(), KernelError> {
    let device = get_device(device_id)?;
    device.cache.lock().write_block(block_num)?;
    Ok(())
}

pub fn commit_all(device_id: DeviceID) -> Result<(), KernelError> {
    let device = get_device(device_id)?;
    device.cache.lock().commit()?;
    Ok(())
}

pub fn get_buf_size(device_id: DeviceID) -> Result<usize, KernelError> {
    let device = get_device(device_id)?;
    let size = device.cache.lock().block_size();
    Ok(size)
}

pub fn set_buf_size(device_id: DeviceID, size: usize) -> Result<(), KernelError> {
    let device = get_device(device_id)?;
    let result = device.cache.lock().set_block_size(size);
    result
}



fn get_device(device_id: DeviceID) -> Result<BlockDeviceEntry, KernelError> {
    let DeviceID(driver_id, subdevice_id) = device_id;
    let mut drivers_list = BLOCK_DRIVERS.lock();
    let driver = drivers_list.get_mut(driver_id as usize).ok_or(KernelError::NoSuchDevice)?;
    let device = driver.devices.get_mut(subdevice_id as usize).ok_or(KernelError::NoSuchDevice)?;
    Ok(device.clone())
}


impl BlockDriver {
    pub const fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            devices: Vec::new(),
        }
    }

    pub fn add_device(&mut self, driver_id: DriverID, dev: Box<dyn BlockOperations>) -> Result<MinorDeviceID, KernelError> {
        let device_id = self.devices.len() as MinorDeviceID;
        self.devices.push(Arc::new(BlockDevice::new(DeviceID(driver_id, device_id), dev)));
        Ok(device_id)
    }
}

impl BlockDevice {
    pub fn new(device_id: DeviceID, dev: Box<dyn BlockOperations>) -> Self {
        Self {
            dev: Spinlock::new(dev),
            cache: Spinlock::new(BufCache::new(device_id, 1024)),
        }
    }
}


pub(super) fn buffered_read(cache: &mut BufCache, buffer: &mut [u8], offset: u64) -> Result<usize, KernelError> {
    let block_size = cache.block_size() as u64;

    let mut buffer_start = 0;
    let mut buffer_remain = buffer.len() as u64;
    let mut block_num = (offset / block_size) as BlockNum;
    let mut block_start = offset % block_size;
    while buffer_remain > 0 {
        let block_end = if buffer_remain > block_size - block_start { block_size } else { buffer_remain };
        let entry = cache.get_block(block_num)?;
        buffer[buffer_start..].copy_from_slice(&entry.lock()[block_start as usize..block_end as usize]);

        buffer_remain = buffer_remain.saturating_sub(block_size - block_start);
        buffer_start += (block_size - block_start) as usize;
        block_num += 1;
        block_start = 0;
    }
    Ok(0)
}

pub(super) fn raw_read(device_id: DeviceID, buffer: &mut [u8], offset: u64) -> Result<usize, KernelError> {
    let device = get_device(device_id)?;
    let result = device.dev.lock().read(buffer, offset);
    result
}

pub(super) fn raw_write(device_id: DeviceID, buffer: &[u8], offset: u64) -> Result<usize, KernelError> {
    let device = get_device(device_id)?;
    let result = device.dev.lock().write(buffer, offset);
    result
}

