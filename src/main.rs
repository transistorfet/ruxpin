#![no_std]
#![no_main]

mod arch;
mod mm;
mod printk;

use core::fmt::Write;
use core::panic::PanicInfo;

use crate::mm::kmalloc::{init_kernel_heap, kmalloc, kmfree};


#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    printk!("Kernel started\n");

    //printk!("CurrentEL: {:x}\n", unsafe { get_current_el() });

    //unsafe { _trigger_illegal_instruction(); }
    //let mut big_addr: u64 = 8 * 1024 * 1024 * 1024 * 1024;
    //unsafe { core::ptr::read_volatile(big_addr as *mut u64) };

    init_kernel_heap(0x100000 as *mut i8, 0x100000);

    unsafe {
        let ptr = kmalloc(1024);
        printk!("Alloc: {:x}\n", ptr as usize);
        kmfree(ptr);
        let ptr = kmalloc(1024);
        printk!("Alloc2: {:x}\n", ptr as usize);
    }

    printk!("Looping\n");
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        printk!("Rust Panic: {:?}\n", s);
    } else {
        printk!("Rust Panic\n");
    }

    loop {}
}

#[no_mangle]
pub extern "C" fn fatal_error(esr: i64, elr: i64) -> ! {
    printk!("Fatal Error: ESR: 0x{:x}, ELR: 0x{:x}\n", esr, elr);

    loop {}
}

