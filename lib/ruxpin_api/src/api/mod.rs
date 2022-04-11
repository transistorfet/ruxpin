
use crate::syscall_encode;
use crate::arch::execute_syscall;
use crate::types::{FileDesc, ApiError};
use crate::syscalls::{SyscallRequest, SyscallFunction};


pub fn write(file: FileDesc, buffer: &[u8]) -> Result<usize, ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, file: FileDesc);
    syscall_encode!(syscall, i, buffer: &[u8]);
    syscall.function = SyscallFunction::Write;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(syscall.result),
        true => Err(ApiError::SomethingWentWrong),
    }
}

