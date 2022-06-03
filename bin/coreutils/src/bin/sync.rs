#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::sync;


#[no_mangle]
pub fn main() {
    sync().unwrap();
}

