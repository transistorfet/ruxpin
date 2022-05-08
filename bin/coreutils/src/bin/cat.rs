#![no_std]
#![no_main]

use core::str;

extern crate ruxpin_app;

use ruxpin_api::{print, println};
use ruxpin_api::api::{exit, open, close, read};
use ruxpin_api::types::{OpenFlags, FileAccess};

use ruxpin_app::env;


#[no_mangle]
pub fn main() {
    let mut args = env::args();
    let filename = match args.nth(1) {
        Some(filename) => filename,
        None => {
            println!("Usage: cat <filename>");
            exit(0);
        },
    };

    let file = open(filename, OpenFlags::ReadOnly, FileAccess::DefaultDir).unwrap();
    loop {
        let mut buffer = [0; 512];
        if read(file.clone(), &mut buffer).unwrap() != 0 {
            print!("{}", str::from_utf8(&buffer).unwrap());
        } else {
            break;
        }
    }
    close(file).unwrap();
    println!("");
}

