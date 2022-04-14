#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::api::{open, close, read, write};
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess};

#[no_mangle]
pub fn main() {
    write(FileDesc(0), b"a really cool message that I'd like to see\n").unwrap();

    let file = open("/mnt/test2", OpenFlags::ReadOnly, FileAccess::DefaultFile).unwrap();
    let mut data = [0; 100];
    let nbytes = read(file, &mut data).unwrap();
    write(FileDesc(0), &data[0..nbytes]).unwrap();
    close(file).unwrap();

    loop {
        let mut data = [0; 1];
        let nbytes = read(FileDesc(0), &mut data[..]).unwrap();
        if nbytes > 0 {
            write(FileDesc(0), &data[..]).unwrap();
            if data[0] == '\r' as u8 {
                break;
            }
        }
    }

    write(FileDesc(0), b"done\n").unwrap();

    loop {
    }
}

