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

use crate::mm::kmalloc::{init_kernel_heap};
use crate::mm::vmalloc::{init_virtual_memory};
use crate::proc::process::{init_processes, create_test_process};

use crate::types::BlockDriver;
use crate::drivers::arm::SystemTimer;
use crate::drivers::arm::GenericInterruptController;
use crate::drivers::raspberrypi::emmc::EmmcDevice;

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    console::Console::init();

    printkln!("starting kernel...");

    //printk!("CurrentEL: {:x}\n", unsafe { get_current_el() });

    //unsafe { _trigger_illegal_instruction(); }
    //let mut big_addr: u64 = 8 * 1024 * 1024 * 1024 * 1024;
    //unsafe { core::ptr::read_volatile(big_addr as *mut u64) };

    unsafe {
        init_kernel_heap(0x20_0000 as *mut u8, 0x100_0000 as *mut u8);
        init_virtual_memory(PhysicalAddress::from(0x100_0000), PhysicalAddress::from(0x1000_0000));
    }


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
    init_processes();

    create_test_process();

    printkln!("Error starting processes... halting...");
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    printkln!("Rust Panic: {}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn fatal_error(elr: i64, esr: i64, far: i64) -> ! {
    printkln!("Fatal Error: ESR: {:#x}, FAR: {:#x}, ELR: {:#x}", esr, far, elr);

    loop {}
}

mod console {
    pub use crate::drivers::raspberrypi::console::*;
}

