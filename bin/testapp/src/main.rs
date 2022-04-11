#![no_std]
#![no_main]

use core::panic::PanicInfo;

use ruxpin_api::api::write;
use ruxpin_api::types::FileDesc;

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    loop {
        write(FileDesc(0), b"a really cool message that I'd like to see").unwrap();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

