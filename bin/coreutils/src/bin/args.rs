#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::println;

use ruxpin_app::env;


#[no_mangle]
pub fn main() {
    let mut args = env::args();

    while let Some(arg) = args.next() {
        println!(">>> {:?}", arg);
    }
}

