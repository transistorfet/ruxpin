#![no_std]
#![no_main]

use core::str;

extern crate ruxpin_app;

use ruxpin_api::{print, println};
use ruxpin_api::api::{exit, fork, exec, open, close, read, write, waitpid};
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess, ApiError};


fn read_input(data: &mut [u8]) -> Result<usize, ApiError> {
    let mut i = 0;
    print!("% ");
    loop {
        let nbytes = read(FileDesc(0), &mut data[i..])?;
        if nbytes > 0 {
            write(FileDesc(0), &data[i..i + nbytes])?;
            i += nbytes;
            if data[i - 1] == '\r' as u8 {
                data[i - 1] = '\n' as u8;
                println!("");
                return Ok(i);
            }
        }
    }
}

fn get_next_word<'a>(input: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    for i in 0..input.len() {
        if input[i] == ' ' as u8 || input[i] == '\n' as u8 || input[i] == '\r' as u8 {
            return (&input[..i], &input[i + 1..]);
        }
    }
    return (input, &input[input.len() - 1..]);
}


#[no_mangle]
pub fn main() {
    println!("\nStarting shell...");

    let mut data = [0; 256];
    loop {
        let length = read_input(&mut data).unwrap();
        let (first, remain) = get_next_word(&data);
        let command = str::from_utf8(first).unwrap();

        if command == "exit" {
            break;
        }

        if command != "" {
            let mut fullpath = [0; 256];
            (&mut fullpath[..5]).copy_from_slice(b"/bin/");
            (&mut fullpath[5..5 + command.len()]).copy_from_slice(first);
            let command = str::from_utf8(&fullpath[..5 + command.len()]).unwrap();

            println!("executing {}", command);
            let pid = fork().unwrap();
            if pid == 0 {
                exec(command);
            } else {
                println!("child pid is {}", pid);
                let mut status = 0;
                waitpid(pid, &mut status, 0);
            }
        }
    }

    println!("done");

    exit(0);

    println!("didn't exit");
    loop {
    }
}

