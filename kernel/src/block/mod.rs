
use core::mem;
use core::slice;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::sync::Arc;
 
use ruxpin_api::types::{OpenFlags, DeviceID, DriverID, SubDeviceID};

use crate::sync::Spinlock;
use crate::errors::KernelError;

mod bufcache;

use self::bufcache::BufCache;


pub trait BlockOperations: Sync + Send {
    fn open(&mut self, mode: OpenFlags) -> Result<(), KernelError>;
    fn close(&mut self) -> Result<(), KernelError>;
    fn read(&mut self, buffer: &mut [u8], offset: usize) -> Result<usize, KernelError>;
    fn write(&mut self, buffer: &[u8], offset: usize) -> Result<usize, KernelError>;
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

pub fn read(device_id: DeviceID, buffer: &mut [u8], offset: usize) -> Result<usize, KernelError> {
    let device = get_device(device_id)?;
    //let result = device.lock().dev.read(buffer, offset);
    let result = device.cache.lock().read(&mut *device.dev.lock(), buffer, offset);
    result
}

pub fn write(device_id: DeviceID, buffer: &[u8], offset: usize) -> Result<usize, KernelError> {
    let device = get_device(device_id)?;
    let result = device.dev.lock().write(buffer, offset);
    result
}

pub fn read_struct<T>(location: &mut T, device_id: DeviceID, offset: usize) -> Result<(), KernelError> {
    let buffer = unsafe { slice::from_raw_parts_mut(location as *mut T as *mut u8, mem::size_of::<T>()) };
    read(device_id, buffer, offset)?;
    Ok(())
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

