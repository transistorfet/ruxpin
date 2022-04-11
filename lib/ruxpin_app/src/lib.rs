#![no_std]
#![no_main]

use core::panic::PanicInfo;

extern "Rust" {
    fn main();
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    unsafe {
        main();
    }
    0
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

