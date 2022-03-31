#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

pub mod arch;
pub mod fs;
pub mod mm;
pub mod proc;
pub mod config;
pub mod misc;
pub mod sync;
pub mod types;
pub mod errors;
pub mod printk;

extern crate alloc;

use core::panic::PanicInfo;

use crate::arch::context::start_multitasking;

use crate::config::register_devices;

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    register_devices().unwrap(); 

    printkln!("scheduler: starting multitasking");
    start_multitasking();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    printkln!("Rust Panic: {}", info);
    loop {}
}

