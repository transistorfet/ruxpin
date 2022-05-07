#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

pub mod env;
pub mod allocator;

use core::panic::PanicInfo;

use ruxpin_api::println;
use ruxpin_api::types::FileDesc;
use ruxpin_api::api::exit;

use crate::env::{Args, Vars};

extern "Rust" {
    fn main();
}

#[no_mangle]
fn _start(argc: isize, argv: *const *const u8, envp: *const *const u8) -> ! {

    Args::set_args(argc, argv);
    Vars::set_vars(envp);

    unsafe {
        main();
    }
    exit(0);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Rust Panic: {}", info);
    exit(-1);
}

