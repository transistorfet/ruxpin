#![no_std]
#![no_main]

use core::panic::PanicInfo;

use ruxpin_api::println;
use ruxpin_api::types::FileDesc;
use ruxpin_api::api::{exit, write};

extern "Rust" {
    fn main();
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> ! {
    unsafe {
        main();
    }
    exit(0);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Rust Panic: {}", info);
    exit(-1);
}

