#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::{println, rename, exit};

use ruxpin_app::env;


#[no_mangle]
pub fn main() {
    let mut args = env::args();
    let (source, dest) = match (args.nth(1), args.next()) {
        (Some(source), Some(dest)) => (source, dest),
        _ => {
            println!("Usage: mv <source> <destination>");
            exit(0);
        },
    };

    match rename(source, dest) {
        Ok(()) => {
            println!("file renamed");
        },
        Err(err) => {
            println!("Error: {:?}", err);
        },
    }
}

