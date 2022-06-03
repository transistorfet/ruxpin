#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::{print, println};

use ruxpin_app::env;


#[no_mangle]
pub fn main() {
    let mut args = env::args();
    args.next();        // Skip the command argument

    while let Some(arg) = args.next() {
        print!("{} ", arg);
    }
    println!("");
}

