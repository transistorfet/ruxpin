#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::println;
use ruxpin_api::api::{open, close, readdir};
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess, DirEntry};

use ruxpin_app::env;


#[no_mangle]
pub fn main() {
    let mut args = env::args();

    while let Some(arg) = args.next() {
        println!(">>> {:?}", arg);
    }

    let mut args = env::args();
    let dirname = args.nth(1).unwrap_or(".");

    let file = open(dirname, OpenFlags::ReadOnly, FileAccess::DefaultDir).unwrap();
    loop {
        let mut dirent = DirEntry::new_empty();
        if readdir(file.clone(), &mut dirent).unwrap() {
            println!("{}", dirent.as_str());
        } else {
            break;
        }
    }
    close(file).unwrap();
}

