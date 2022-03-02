#![no_std]
#![no_main]

mod arch;

use core::fmt::Write;
use core::panic::PanicInfo;

use arch::aarch64::console::SimpleConsole;


#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


static HELLO: &str = "Hello World!\nAnd this is the thing that involves the stuff and it's really cooled and i like it to bits\n";

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    let mut console = SimpleConsole {};

    console.write_str(HELLO).unwrap();

    loop {}
}

