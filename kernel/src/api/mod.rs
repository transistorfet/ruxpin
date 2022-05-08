
use ruxpin_api::types::{Pid, FileDesc, OpenFlags, FileAccess, DirEntry, ApiError};
use ruxpin_api::syscall_decode;
use ruxpin_api::syscalls::{SyscallRequest, SyscallFunction};

use crate::printkln;
use crate::api::file::*;
use crate::api::proc::*;
use crate::errors::KernelError;
use crate::arch::context::Context;
use crate::proc::scheduler::{get_current, suspend, check_restart_syscall};

mod file;
mod proc;

pub fn handle_syscall() {
    //crate::printkln!("A SYSCALL for {:?}!", syscall.function);

    let mut syscall = Context::syscall_from_current_context();
    get_current().try_lock().unwrap().syscall = syscall.clone();
    process_syscall(&mut syscall);
    check_restart_syscall();
}

pub fn process_syscall(syscall: &mut SyscallRequest) {
    if syscall.function == SyscallFunction::Exec {
        let mut i = 0;
        syscall_decode!(syscall, i, path: &str);
        syscall_decode!(syscall, i, args: &[&str]);
        syscall_decode!(syscall, i, envp: &[&str]);
        if let Err(err) = syscall_exec(path, args, envp) {
            store_result(syscall, Err(err));
            Context::write_syscall_result_to_current_context(syscall);
        }
        // Return without setting the return value, which would overwrite the
        // command line arguments written to the context by the exec loader
        return;
    }

    match syscall.function {
        SyscallFunction::Exit => {
            let mut i = 0;
            syscall_decode!(syscall, i, status: isize);
            let result = syscall_exit(status);
            store_result(syscall, result.map(|_| 0));
            //self::proc::handle_syscall_exit(syscall);
        },

        SyscallFunction::Fork => {
            let result = syscall_fork();
            store_result(syscall, result.map(|ret| ret as usize));
            //self::proc::handle_syscall_fork(syscall);
        },

        //SyscallFunction::Exec => {
        //    let mut i = 0;
        //    syscall_decode!(syscall, i, path: &str);
        //    syscall_decode!(syscall, i, args: &[&str]);
        //    syscall_decode!(syscall, i, envp: &[&str]);
        //    let result = syscall_exec(path, args, envp);
        //    store_result(syscall, result.map(|_| 0));
        //},

        SyscallFunction::WaitPid => {
            let mut i = 0;
            syscall_decode!(syscall, i, pid: Pid);
            syscall_decode!(syscall, i, status: &mut isize);
            syscall_decode!(syscall, i, options: usize);
            let result = syscall_waitpid(pid, status, options);
            store_result(syscall, result.map(|ret| ret as usize));
        },

        SyscallFunction::Sbrk => {
            let mut i = 0;
            syscall_decode!(syscall, i, increment: usize);
            let result = syscall_sbrk(increment);
            store_result(syscall, result.map(|ret| ret as usize));
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
        SyscallFunction::ReadDir => {
            let mut i = 0;
            syscall_decode!(syscall, i, file: FileDesc);
            syscall_decode!(syscall, i, dirent: &mut DirEntry);
            let result = syscall_readdir(file, dirent);
            store_result(syscall, result.map(|r| r as usize));
        },
        _ => {
            printkln!("syscall: invalid function number: {}", syscall.function as usize);
            store_result(syscall, Err(KernelError::BadSystemCall));
        }
    }

    Context::write_syscall_result_to_current_context(syscall);
}

pub fn store_result(syscall: &mut SyscallRequest, result: Result<usize, KernelError>) {
    match result {
        Ok(value) => {
            syscall.error = false;
            syscall.result = value;
        },
        Err(value) => {
            if value == KernelError::SuspendProcess {
                suspend(get_current());
            }

            syscall.error = true;
            syscall.result = ApiError::from(value) as usize;
        },
    }
}

