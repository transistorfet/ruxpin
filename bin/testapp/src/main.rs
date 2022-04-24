#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::println;
use ruxpin_api::api::{exit, exec, open, close, read, write};
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess};

#[no_mangle]
pub fn main() {
    println!("a really cool message that I'd like to see");

    let file = open("/mnt/test2", OpenFlags::ReadOnly, FileAccess::DefaultFile).unwrap();
    let mut data = [0; 100];
    let nbytes = read(file, &mut data).unwrap();
    write(FileDesc(0), &data[0..nbytes]).unwrap();
    close(file).unwrap();

    let mut i = 0;
    let mut data = [0; 256];
    loop {
        let nbytes = read(FileDesc(0), &mut data[i..]).unwrap();
        if nbytes > 0 {
            i += nbytes;
            if data[i - 1] == '\r' as u8 {
                data[i - 1] = '\n' as u8;
                write(FileDesc(0), b"read in ").unwrap();
                write(FileDesc(0), &data[0..i]).unwrap();

                if &data[0..i] == b"exit\n" {
                    break;
                }

                if &data[0..i] == b"run\n" {
                    println!("executing self");
                    exec("/mnt/bin/testapp", &[], &[]);
                }

                i = 0;
            }
        }
    }

    println!("done");

    exit(0);

    println!("didn't exit");
    loop {
    }
}

