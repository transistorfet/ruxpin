#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::{println, unlink, exit};

use ruxpin_app::env;


#[no_mangle]
pub fn main() {
    let mut args = env::args();
    let filename = match args.nth(1) {
        Some(filename) => filename,
        None => {
            println!("Usage: rm <filename>");
            exit(0);
        },
    };

    match unlink(filename) {
        Ok(()) => {
            println!("file deleted");
        },
        Err(err) => {
            println!("Error: {:?}", err);
        },
    }
}

