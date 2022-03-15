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


#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    printkln!("Kernel started");

    //printk!("CurrentEL: {:x}\n", unsafe { get_current_el() });

    //unsafe { _trigger_illegal_instruction(); }
    //let mut big_addr: u64 = 8 * 1024 * 1024 * 1024 * 1024;
    //unsafe { core::ptr::read_volatile(big_addr as *mut u64) };

    unsafe {
        init_kernel_heap(0x20_0000 as *mut u8, 0x100_0000 as *mut u8);
        init_virtual_memory(0x100_0000 as *mut u8, 0x1000_0000 as *mut u8);
    }

    crate::drivers::arm::gic::init_gic();
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
pub extern "C" fn fatal_error(esr: i64, elr: i64) -> ! {
    printkln!("Fatal Error: ESR: 0x{:x}, ELR: 0x{:x}", esr, elr);

    loop {}
}

mod console {
    pub use crate::drivers::raspberrypi::console::*;
}

