#![no_std]
#![no_main]

mod arch;

use core::fmt::Write;
use core::panic::PanicInfo;

use arch::console::Console;

static HELLO: &str = "Hello World!\nAnd this is the thing that involves the stuff and it's really cooled and i like it to bits\n";

extern {
    fn _trigger_illegal_instruction() -> !;
    fn _get_current_el() -> i64;
}

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    let mut console = Console {};

    console.write_str(HELLO).unwrap();

    //console.write_fmt(format_args!("{:x}", unsafe { _get_current_el() }));
    //console.write_str("\n");

    //unsafe { _trigger_illegal_instruction(); }
    //let mut big_addr: u64 = 8 * 1024 * 1024 * 1024 * 1024;
    //unsafe { core::ptr::read_volatile(big_addr as *mut u64) };

    console.write_str("aaand loop\n");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let mut console = Console {};
    console.write_str("Rust Panic\n").unwrap();

    loop {}
}

#[no_mangle]
pub extern "C" fn fatal_error() -> ! {
    let mut console = Console {};
    console.write_str("Fatal Error\n").unwrap();

    loop {}
}

