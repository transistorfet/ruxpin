
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess};
use ruxpin_api::syscall_decode;
use ruxpin_api::syscalls::{SyscallRequest, SyscallFunction};

use crate::api::file::*;
use crate::api::proc::*;
use crate::errors::KernelError;
use crate::proc::process::get_current_process;

mod file;
mod proc;

pub fn handle_syscall(syscall: &mut SyscallRequest) {
    get_current_process().lock().syscall = syscall.clone();

    match syscall.function {
        SyscallFunction::Exit => {
            let result = syscall_exit(syscall.args[0]);
            store_result(syscall, result.map(|_| 0));
        },

        SyscallFunction::Fork => {
            let result = syscall_fork(syscall.args[0]);
            store_result(syscall, result.map(|ret| ret as usize));
        },

        SyscallFunction::Exec => {
            let mut i = 0;
            syscall_decode!(syscall, i, path: &str);
            let result = syscall_exec(path);
            store_result(syscall, result.map(|_| 0));
        },

        SyscallFunction::Open => {
            let mut i = 0;
            syscall_decode!(syscall, i, path: &str);
            syscall_decode!(syscall, i, flags: OpenFlags);
            syscall_decode!(syscall, i, access: FileAccess);
            let result = syscall_open(path, flags, access);
            store_result(syscall, result.map(|ret| ret.0));
        },
        SyscallFunction::Close => {
            let mut i = 0;
            syscall_decode!(syscall, i, file: FileDesc);
            let result = syscall_close(file);
            store_result(syscall, result.map(|_| 0));
        },
        SyscallFunction::Read => {
            let mut i = 0;
            syscall_decode!(syscall, i, file: FileDesc);
            syscall_decode!(syscall, i, buffer: &mut [u8]);
            let result = syscall_read(file, buffer);
            store_result(syscall, result);
        },
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

