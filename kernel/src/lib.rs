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
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::context::start_multitasking;
use crate::errors::KernelError;

static BOOT_CORE_INITIALIZED: AtomicBool = AtomicBool::new(false);

extern "Rust" {
    fn register_devices() -> Result<(), KernelError>;
}

#[no_mangle]
pub extern "C" fn boot_core_start() -> ! {
    unsafe {
        register_devices().unwrap();
    }

    BOOT_CORE_INITIALIZED.store(true, Ordering::Release);

    notice!("scheduler: starting multitasking");
    start_multitasking()
}

#[no_mangle]
pub extern "C" fn non_boot_core_start() -> ! {
    loop {
        if BOOT_CORE_INITIALIZED.load(Ordering::Acquire) {
            break;
        }
    }

    //notice!("cpu {}: booting secondary core", arch::cpu_id());
    //let file = crate::fs::vfs::open(None, "/", ruxpin_types::OpenFlags::ReadOnly, ruxpin_types::FileAccess::DefaultDir, 0);
    //notice!("done");

    // TODO this does nothing for the moment
    arch::loop_forever()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("Rust Panic: {}", info);
    arch::loop_forever()
}

