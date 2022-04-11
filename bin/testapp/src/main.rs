#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::api::write;
use ruxpin_api::types::FileDesc;

#[no_mangle]
pub fn main() {
    loop {
        write(FileDesc(0), b"a really cool message that I'd like to see").unwrap();
    }
}

