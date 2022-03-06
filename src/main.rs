#![no_std]
#![no_main]

mod arch;
mod printk;

use core::fmt::Write;
use core::panic::PanicInfo;

use arch::console::Console;

static HELLO: &str = "Hello World!\nAnd this is the thing that involves the stuff and it's really cooled and i like it to bits\n";

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    let mut console = Console {};

    console.write_str(HELLO).unwrap();

    //printk!("CurrentEL: {:x}\n", unsafe { get_current_el() });

    //unsafe { _trigger_illegal_instruction(); }
    //let mut big_addr: u64 = 8 * 1024 * 1024 * 1024 * 1024;
    //unsafe { core::ptr::read_volatile(big_addr as *mut u64) };

    console.write_str("aaand loop\n").unwrap();
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    printk!("Rust Panic\n");

    loop {}
}

#[no_mangle]
pub extern "C" fn fatal_error(esr: i64, elr: i64) -> ! {
    printk!("Fatal Error: ESR: 0x{:x}, ELR: 0x{:x}\n", esr, elr);

    loop {}
}

