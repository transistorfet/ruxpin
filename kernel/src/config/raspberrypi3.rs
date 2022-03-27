 
use crate::printkln;
use crate::errors::KernelError;
use crate::types::{CharDriver, BlockDriver};
use crate::arch::types::PhysicalAddress;

use crate::proc::process::init_processes;
use crate::mm::kmalloc::init_kernel_heap;
use crate::mm::vmalloc::init_virtual_memory;


#[path = "../drivers/arm/mod.rs"]
pub mod arm;

#[path = "../drivers/raspberrypi/mod.rs"]
pub mod raspberrypi;

use self::arm::SystemTimer;
use self::arm::GenericInterruptController;
use self::raspberrypi::emmc::EmmcDevice;

pub mod console {
    pub use super::raspberrypi::console::{ConsoleDevice, get_console_device};
}

pub fn register_devices() -> Result<(), KernelError> {
    console::get_console_device().init()?;

    printkln!("starting kernel...");

    // Since data in the heap could be accessed at any time, we use the kernel address space so that TTBR1 is always used for lookups
    unsafe { init_kernel_heap(0xffff_0000_0020_0000 as *mut u8, 0xffff_0000_0100_0000 as *mut u8) };

    init_virtual_memory(PhysicalAddress::from(0x100_0000), PhysicalAddress::from(0x1000_0000));
    init_processes();

    let block_device: &mut dyn BlockDriver = &mut EmmcDevice{};
    printkln!("emmc: initializing");
    block_device.init().unwrap();
    let mut data = [0; 1024];
    block_device.read(&mut data, 0).unwrap();
    unsafe {
        crate::printk::printk_dump(&data as *const u8, 1024);
    }

    SystemTimer::init();
    GenericInterruptController::init();

    Ok(())
}

