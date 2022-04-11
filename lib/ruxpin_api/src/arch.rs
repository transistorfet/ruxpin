
use core::arch::asm;

use crate::syscalls::SyscallRequest;

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn execute_syscall(syscall: &mut SyscallRequest) {
    unsafe {
        let mut result: usize;
        let mut error: usize;
        asm!(
            "svc    #1",
            "mov    {result}, x0",
            "mov    {error}, x1",
            result = out(reg) result,
            error = out(reg) error,
            in("x0") syscall.args[0],
            in("x1") syscall.args[1],
            in("x2") syscall.args[2],
            in("x3") syscall.args[3],
            in("x4") syscall.args[4],
            in("x5") syscall.args[5],
            in("x6") syscall.function as usize,
        );
        syscall.result = result;
        syscall.error = error != 0;
    }
}

