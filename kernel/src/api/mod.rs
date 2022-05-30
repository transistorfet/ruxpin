
use ruxpin_syscall::{SyscallRequest, SyscallFunction};
use ruxpin_types::ApiError;

use crate::error;
use crate::arch::context::Context;
use crate::proc::scheduler::{get_current, check_restart_syscall};

mod file;
mod proc;


pub fn handle_syscall() {
    //crate::info!("A SYSCALL for {:?}!", syscall.function);

    let mut syscall = Context::syscall_from_current_context();
    get_current().try_lock().unwrap().syscall = syscall.clone();
    process_syscall(&mut syscall);
    check_restart_syscall();
}

pub fn process_syscall(syscall: &mut SyscallRequest) {
    let current_proc = get_current();

    if syscall.function == SyscallFunction::Exec {
        self::proc::handle_syscall_exec(syscall);
        if syscall.error {
            Context::write_syscall_result_to_current_context(syscall);
        }
        // Return without setting the return value, which would overwrite the
        // command line arguments written to the context by the exec loader
        return;
    }

    match syscall.function {
        SyscallFunction::Exit => {
            self::proc::handle_syscall_exit(syscall);
        },

        SyscallFunction::Fork => {
            self::proc::handle_syscall_fork(syscall);
        },

        //SyscallFunction::Exec => {
        //    self::proc::handle_syscall_exec(syscall);
        //},

        SyscallFunction::WaitPid => {
            self::proc::handle_syscall_waitpid(syscall);
        },

        SyscallFunction::Sbrk => {
            self::proc::handle_syscall_sbrk(syscall);
        },

        SyscallFunction::Open => {
            self::file::handle_syscall_open(syscall);
        },
        SyscallFunction::Close => {
            self::file::handle_syscall_close(syscall);
        },
        SyscallFunction::Read => {
            self::file::handle_syscall_read(syscall);
        },
        SyscallFunction::Write => {
            self::file::handle_syscall_write(syscall);
        },
        SyscallFunction::ReadDir => {
            self::file::handle_syscall_readdir(syscall);
        },
        SyscallFunction::Unlink => {
            self::file::handle_syscall_unlink(syscall);
        },
        SyscallFunction::Rename => {
            self::file::handle_syscall_rename(syscall);
        },
        SyscallFunction::MkDir => {
            self::file::handle_syscall_mkdir(syscall);
        },
        SyscallFunction::GetCwd => {
            self::file::handle_syscall_getcwd(syscall);
        },
        _ => {
            error!("syscall: invalid function number: {}", syscall.function as usize);
            syscall.store_result(Err(ApiError::BadSystemCall));
        }
    }

    current_proc.try_lock().unwrap().context.write_syscall_result(syscall);
}

