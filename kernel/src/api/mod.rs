
use ruxpin_api::types::{FileDesc};
use ruxpin_api::syscall_decode;
use ruxpin_api::syscalls::{SyscallRequest, SyscallFunction};

use crate::api::file::*;
use crate::errors::KernelError;

mod file;

pub fn handle_syscall(syscall: &mut SyscallRequest) {
    match syscall.function {
        SyscallFunction::Write => {
            let mut i = 0;
            syscall_decode!(syscall, i, file: FileDesc);
            syscall_decode!(syscall, i, buffer: &[u8]);
            let result = syscall_write(file, buffer);
            store_result(syscall, result);
        },
        _ => panic!("syscall: invalid function number: {}", syscall.function as usize),
    }
}

pub fn store_result(syscall: &mut SyscallRequest, result: Result<usize, KernelError>) {
    match result {
        Ok(value) => {
            syscall.error = false;
            syscall.result = value;
        },
        Err(value) => {
            syscall.error = true;
            syscall.result = value as usize;
        },
    }
}

