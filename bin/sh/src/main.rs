#![no_std]
#![no_main]

extern crate ruxpin_app;

use ruxpin_api::println;
use ruxpin_api::api::{exit, fork, exec, open, close, read, write, waitpid};
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess, ApiError};


fn read_input(data: &mut [u8]) -> Result<usize, ApiError> {
    let mut i = 0;
    loop {
        let nbytes = read(FileDesc(0), &mut data[i..])?;
        if nbytes > 0 {
            i += nbytes;
            if data[i - 1] == '\r' as u8 {
                data[i - 1] = '\n' as u8;
                write(FileDesc(0), b"read in ")?;
                write(FileDesc(0), &data[0..i])?;
                return Ok(i);
            }
        }
    }
}


#[no_mangle]
pub fn main() {
    println!("Starting shell...");

    let mut data = [0; 256];
    loop {
        let length = read_input(&mut data).unwrap();

        if &data[0..length] == b"exit\n" {
            break;
        }

        if &data[0..length] == b"run\n" {
            println!("executing testapp");
            let pid = fork().unwrap();
            if pid == 0 {
                exec("/mnt/bin/ls");
            } else {
                println!("child pid is {}", pid);
                let mut status = 0;
                waitpid(pid, &mut status, 0).unwrap();
            }
        }
    }

    println!("done");

    exit(0);

    println!("didn't exit");
    loop {
    }
}

