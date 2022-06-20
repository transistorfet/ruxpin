#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

pub mod api;
pub mod arch;
pub mod block;
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
use crate::errors::KernelError;

extern "Rust" {
    fn register_devices() -> Result<(), KernelError>;
}

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    unsafe {
        register_devices().unwrap();
    }

    notice!("scheduler: starting multitasking");
    start_multitasking();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("Rust Panic: {}", info);
    arch::loop_forever();
}

