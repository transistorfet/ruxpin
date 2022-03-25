#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

mod arch;
mod mm;
mod proc;
mod drivers;
mod types;
mod errors;
mod printk;

extern crate alloc;

use core::panic::PanicInfo;

use crate::arch::types::PhysicalAddress;
use crate::arch::context::start_multitasking;

use crate::proc::process::init_processes;
use crate::mm::kmalloc::init_kernel_heap;
use crate::mm::vmalloc::init_virtual_memory;

use crate::types::BlockDriver;
use crate::drivers::arm::SystemTimer;
use crate::drivers::arm::GenericInterruptController;
use crate::drivers::raspberrypi::emmc::EmmcDevice;

mod console {
    pub use crate::drivers::raspberrypi::console::*;
}


#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    console::Console::init();

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


    printkln!("scheduler: starting multitasking");
    start_multitasking();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    printkln!("Rust Panic: {}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn fatal_error(elr: u64, esr: u64, far: u64) -> ! {
    printkln!("Fatal Error: ESR: {:#x}, FAR: {:#x}, ELR: {:#x}", esr, far, elr);
    loop {}
}

