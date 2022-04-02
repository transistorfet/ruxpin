
use alloc::vec::Vec;
use alloc::boxed::Box;
 
use ruxpin_api::types::{OpenFlags, DeviceID, DriverID, SubDeviceID};

use crate::sync::Spinlock;
use crate::errors::KernelError;


pub trait CharOperations: Sync + Send {
    fn open(&mut self, mode: OpenFlags) -> Result<(), KernelError>;
    fn close(&mut self) -> Result<(), KernelError>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, KernelError>;
    fn write(&mut self, buffer: &[u8]) -> Result<usize, KernelError>;
    //int (*ioctl)(devminor_t minor, unsigned int request, void *argp, uid_t uid);
    //int (*poll)(devminor_t minor, int events);
    //offset_t (*seek)(devminor_t minor, offset_t position, int whence, offset_t offset);
}



static TTY_DRIVERS: Spinlock<Vec<CharDriver>> = Spinlock::new(Vec::new());


pub fn register_tty_driver(prefix: &'static str) -> Result<DriverID, KernelError> {
    let driver_id = TTY_DRIVERS.lock().len() as DriverID;
    TTY_DRIVERS.lock().push(CharDriver::new(prefix));
    Ok(driver_id)
}

pub fn register_tty_device(driver_id: DriverID, dev: Box<dyn CharOperations>) -> Result<SubDeviceID, KernelError> {
    let mut driver_list = TTY_DRIVERS.lock();
    let driver = driver_list.get_mut(driver_id as usize).ok_or(KernelError::NoSuchDevice)?;
    driver.add_device(dev)
}

pub fn read(device: DeviceID, buffer: &mut [u8]) -> Result<usize, KernelError> {
    let DeviceID(driver_id, subdevice_id) = device;
    let mut drivers_list = TTY_DRIVERS.lock();
    let driver = drivers_list.get_mut(driver_id as usize).ok_or(KernelError::NoSuchDevice)?;
    let device = driver.devices.get_mut(subdevice_id as usize).ok_or(KernelError::NoSuchDevice)?;
    device.dev.read(buffer)
}

pub fn write(device: DeviceID, buffer: &[u8]) -> Result<usize, KernelError> {
    let DeviceID(driver_id, subdevice_id) = device;
    let mut drivers_list = TTY_DRIVERS.lock();
    let driver = drivers_list.get_mut(driver_id as usize).ok_or(KernelError::NoSuchDevice)?;
    let device = driver.devices.get_mut(subdevice_id as usize).ok_or(KernelError::NoSuchDevice)?;
    device.dev.write(buffer)
}


pub struct CharDriver {
    prefix: &'static str,
    devices: Vec<TtyDevice>,
}

impl CharDriver {
    pub const fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            devices: Vec::new(),
        }
    }

    pub fn add_device(&mut self, dev: Box<dyn CharOperations>) -> Result<SubDeviceID, KernelError> {
        let device_id = self.devices.len() as SubDeviceID;
        self.devices.push(TtyDevice::new(dev));
        Ok(device_id)
    }
}

pub struct TtyDevice {
    dev: Box<dyn CharOperations>,
}

impl TtyDevice {
    pub fn new(dev: Box<dyn CharOperations>) -> Self {
        Self {
            dev,
        }
    }
}

