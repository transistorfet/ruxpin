#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

mod arch;
mod mm;
mod proc;
mod drivers;
mod errors;
mod printk;

extern crate alloc;

use core::panic::PanicInfo;

use crate::mm::kmalloc::{init_kernel_heap};
use crate::mm::vmalloc::{init_virtual_memory};
use crate::proc::process::{init_processes, create_test_process};

use crate::drivers::arm::SystemTimer;
use crate::drivers::arm::GenericInterruptController;

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    console::Console::init();

    printkln!("Kernel started");

    //printk!("CurrentEL: {:x}\n", unsafe { get_current_el() });

    //unsafe { _trigger_illegal_instruction(); }
    //let mut big_addr: u64 = 8 * 1024 * 1024 * 1024 * 1024;
    //unsafe { core::ptr::read_volatile(big_addr as *mut u64) };

    unsafe {
        init_kernel_heap(0x20_0000 as *mut u8, 0x100_0000 as *mut u8);
        init_virtual_memory(0x100_0000 as *mut u8, 0x1000_0000 as *mut u8);
    }

    SystemTimer::init();
    GenericInterruptController::init();
    init_processes();

    create_test_process();

    printkln!("Looping");

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        printkln!("Rust Panic: {:?}", s);
    } else {
        printkln!("Rust Panic");
    }

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

