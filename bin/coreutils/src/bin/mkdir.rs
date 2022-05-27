#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_app::env;
use ruxpin_api::{println, mkdir, exit};
use ruxpin_types::FileAccess;


#[no_mangle]
pub fn main() {
    let mut args = env::args();
    let dirname = match args.nth(1) {
        Some(dirname) => dirname,
        None => {
            println!("Usage: mkdir <dirname>");
            exit(0);
        },
    };

    match mkdir(dirname, FileAccess::DefaultDir) {
        Ok(()) => {
            println!("directory created");
        },
        Err(err) => {
            println!("Error: {:?}", err);
        },
    }
}

