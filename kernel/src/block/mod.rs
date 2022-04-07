
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::boxed::Box;
 
use ruxpin_api::types::{OpenFlags, DeviceID, DriverID, SubDeviceID};

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

pub fn register_block_device(driver_id: DriverID, dev: Box<dyn BlockOperations>) -> Result<SubDeviceID, KernelError> {
    let mut drivers_list = BLOCK_DRIVERS.lock();
    let driver = drivers_list.get_mut(driver_id as usize).ok_or(KernelError::NoSuchDevice)?;
    driver.add_device(dev)
}

pub fn lookup_device(name: &str) -> Result<DeviceID, KernelError> {
    let drivers_list = BLOCK_DRIVERS.lock();
    for (driver_id, driver) in drivers_list.iter().enumerate() {
        if driver.prefix == &name[..driver.prefix.len()] {
            let subdevice_id = name[driver.prefix.len()..].parse::<SubDeviceID>().map_err(|_| KernelError::NoSuchDevice)?;
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
    //let result = device.lock().dev.read(buffer, offset);
    let result = device.cache.lock().read(&mut *device.dev.lock(), buffer, offset);
    result
}

pub fn write(device_id: DeviceID, buffer: &[u8], offset: u64) -> Result<usize, KernelError> {
    let device = get_device(device_id)?;
    let result = device.dev.lock().write(buffer, offset);
    result
}


pub fn get_buf(device_id: DeviceID, block_num: BlockNum) -> Result<CacheArc<Buf>, KernelError> {
    let device = get_device(device_id)?;
    let buf = device.cache.lock().get_block(&mut *device.dev.lock(), block_num)?;
    Ok(buf)
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

    pub fn add_device(&mut self, dev: Box<dyn BlockOperations>) -> Result<SubDeviceID, KernelError> {
        let device_id = self.devices.len() as SubDeviceID;
        self.devices.push(Arc::new(BlockDevice::new(dev)));
        Ok(device_id)
    }
}

impl BlockDevice {
    pub fn new(dev: Box<dyn BlockOperations>) -> Self {
        Self {
            dev: Spinlock::new(dev),
            cache: Spinlock::new(BufCache::new(1024)),
        }
    }
}

