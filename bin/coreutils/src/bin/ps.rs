#![no_std]
#![no_main]

use core::str;
use alloc::format;

extern crate alloc;
extern crate ruxpin_app;

use ruxpin_api::println;
use ruxpin_api::api::{open, close, read, readdir};
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess, DirEntry};


fn print_stat(filename: &str) {
    let mut data = [0; 256];
    let file = open(filename, OpenFlags::ReadOnly, FileAccess::DefaultDir).unwrap();
    let nbytes = read(file.clone(), &mut data).unwrap();
    println!("{}", str::from_utf8(&data[..nbytes]).unwrap());
    close(file).unwrap();
}

#[no_mangle]
pub fn main() {
    let file = open("/proc", OpenFlags::ReadOnly, FileAccess::DefaultDir).unwrap();
    loop {
        let mut dirent = DirEntry::new_empty();
        if readdir(file.clone(), &mut dirent).unwrap() {
            match dirent.as_str().parse::<usize>() {
                Ok(_) => {
/*
                    // TODO change all of this once you have a heap
                    let end_pid = 6 + dirent.as_str().len();
                    let end = end_pid + 5;
                    let mut filename = [0; 128];
                    filename[0..6].copy_from_slice(b"/proc/");
                    filename[6..end_pid].copy_from_slice(dirent.as_str().as_bytes());
                    filename[end_pid..end].copy_from_slice(b"/stat");
                    print_stat(str::from_utf8(&filename[..end]).unwrap());
*/
                    print_stat(&format!("/proc/{}/stat", dirent.as_str()));
                },
                _ => { /* skip non-numeric files */ },
            }
        } else {
            break;
        }
    }
    close(file).unwrap();
}

