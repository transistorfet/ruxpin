#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(int_log)]

pub mod api;
pub mod arch;
pub mod block;
pub mod config;
pub mod errors;
pub mod fs;
pub mod irqs;
pub mod misc;
pub mod mm;
pub mod printk;
pub mod proc;
pub mod sync;
pub mod tasklets;
pub mod tty;

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

