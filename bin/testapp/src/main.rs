#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::api::{open, close, read, write};
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess};

#[no_mangle]
pub fn main() {
    write(FileDesc(0), b"a really cool message that I'd like to see").unwrap();

    let file = open("/mnt/test2", OpenFlags::ReadOnly, FileAccess::DefaultFile).unwrap();
    let mut data = [0; 100];
    read(file, &mut data).unwrap();
    write(FileDesc(0), &data).unwrap();
    close(file).unwrap();

    loop {
    }
}

