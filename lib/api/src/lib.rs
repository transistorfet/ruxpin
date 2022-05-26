#![no_std]

use core::fmt;
use core::fmt::Write;

use ruxpin_syscall_proc::syscall_function;

use ruxpin_types::{Pid, FileDesc, ApiError, OpenFlags, FileAccess, DirEntry};


#[syscall_function(Exit)]
pub fn exit(status: isize) -> ! {}

#[syscall_function(Fork)]
pub fn fork() -> Result<Pid, ApiError> {}

#[syscall_function(Exec)]
pub fn exec(path: &str, args: &[&str], envp: &[&str]) -> ! {}

#[syscall_function(WaitPid)]
pub fn waitpid(pid: Pid, status: &mut isize, options: usize) -> Result<Pid, ApiError> {}

#[syscall_function(Sbrk)]
pub fn sbrk(increment: usize) -> Result<*const u8, ApiError> {}

#[syscall_function(Open)]
pub fn open(path: &str, flags: OpenFlags, access: FileAccess) -> Result<FileDesc, ApiError> {}

#[syscall_function(Close)]
pub fn close(file: FileDesc) -> Result<(), ApiError> {}

#[syscall_function(Read)]
pub fn read(file: FileDesc, buffer: &mut [u8]) -> Result<usize, ApiError> {}

#[syscall_function(Write)]
pub fn write(file: FileDesc, buffer: &[u8]) -> Result<usize, ApiError> {}

#[syscall_function(ReadDir)]
pub fn readdir(file: FileDesc, dirent: &mut DirEntry) -> Result<bool, ApiError> {}

#[syscall_function(Unlink)]
pub fn unlink(path: &str) -> Result<(), ApiError> {}

#[syscall_function(MkDir)]
pub fn mkdir(path: &str, access: FileAccess) -> Result<(), ApiError> {}

#[syscall_function(GetCwd)]
pub fn getcwd(path: &mut [u8]) -> Result<(), ApiError> {}

#[syscall_function(Rename)]
pub fn rename(old_path: &str, new_path: &str) -> Result<(), ApiError> {}


//pub static STDIN: UnbufferedFile = UnbufferedFile(FileDesc(0));
//pub static STDOUT: UnbufferedFile = UnbufferedFile(FileDesc(1));
//pub static STDERR: UnbufferedFile = UnbufferedFile(FileDesc(2));

pub struct UnbufferedFile(pub FileDesc);

impl UnbufferedFile {
    pub fn stdout() -> UnbufferedFile {
        UnbufferedFile(FileDesc(0))
    }
}

impl Write for UnbufferedFile {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(self.0, s.as_bytes()).unwrap();
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => ({
        use core::fmt::Write;
        $crate::UnbufferedFile::stdout().write_fmt(format_args!($($args)*)).unwrap();
    })
}

#[macro_export]
macro_rules! println {
    ($($args:tt)*) => ({
        use core::fmt::Write;
        $crate::UnbufferedFile::stdout().write_fmt(format_args!($($args)*)).unwrap();
        $crate::UnbufferedFile::stdout().write_str("\n").unwrap();
    })
}

/*
pub fn exit(status: isize) -> ! {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, status: isize);
    syscall.function = SyscallFunction::Exit;
    execute_syscall(&mut syscall);

    unsafe { core::hint::unreachable_unchecked(); }
}

pub fn fork() -> Result<Pid, ApiError> {
    let mut syscall: SyscallRequest = Default::default();
    syscall.function = SyscallFunction::Fork;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(syscall.result as Pid),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn exec(path: &str, args: &[&str], envp: &[&str]) {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, path: &str);
    syscall_encode!(syscall, i, args: &[&str]);
    syscall_encode!(syscall, i, envp: &[&str]);
    syscall.function = SyscallFunction::Exec;
    execute_syscall(&mut syscall);
}

pub fn waitpid(pid: Pid, status: &mut isize, options: usize) -> Result<Pid, ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall.function = SyscallFunction::WaitPid;
    syscall_encode!(syscall, i, pid: Pid);
    syscall_encode!(syscall, i, status: &isize);
    syscall_encode!(syscall, i, options: usize);
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(syscall.result as Pid),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn sbrk(increment: usize) -> Result<*const u8, ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall.function = SyscallFunction::Sbrk;
    syscall_encode!(syscall, i, increment: usize);
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(syscall.result as *const u8),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn open(path: &str, flags: OpenFlags, access: FileAccess) -> Result<FileDesc, ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, path: &str);
    syscall_encode!(syscall, i, flags: OpenFlags);
    syscall_encode!(syscall, i, access: FileAccess);
    syscall.function = SyscallFunction::Open;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(FileDesc(syscall.result)),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn close(file: FileDesc) -> Result<(), ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, file: FileDesc);
    syscall.function = SyscallFunction::Close;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(()),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn read(file: FileDesc, buffer: &mut [u8]) -> Result<usize, ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, file: FileDesc);
    syscall_encode!(syscall, i, buffer: &mut [u8]);
    syscall.function = SyscallFunction::Read;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(syscall.result),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn write(file: FileDesc, buffer: &[u8]) -> Result<usize, ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, file: FileDesc);
    syscall_encode!(syscall, i, buffer: &[u8]);
    syscall.function = SyscallFunction::Write;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(syscall.result),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn readdir(file: FileDesc, dirent: &mut DirEntry) -> Result<bool, ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, file: FileDesc);
    syscall_encode!(syscall, i, dirent: &mut DirEntry);
    syscall.function = SyscallFunction::ReadDir;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(syscall.result != 0),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn unlink(path: &str) -> Result<(), ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, path: &str);
    syscall.function = SyscallFunction::Unlink;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(()),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn mkdir(path: &str, access: FileAccess) -> Result<(), ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, path: &str);
    syscall_encode!(syscall, i, access: FileAccess);
    syscall.function = SyscallFunction::MkDir;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(()),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn getcwd(path: &mut [u8]) -> Result<(), ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, path: &mut [u8]);
    syscall.function = SyscallFunction::GetCwd;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(()),
        true => Err(ApiError::from(syscall.result)),
    }
}

pub fn rename(old_path: &str, new_path: &str) -> Result<(), ApiError> {
    let mut i = 0;
    let mut syscall: SyscallRequest = Default::default();
    syscall_encode!(syscall, i, old_path: &str);
    syscall_encode!(syscall, i, new_path: &str);
    syscall.function = SyscallFunction::Rename;
    execute_syscall(&mut syscall);
    match syscall.error {
        false => Ok(()),
        true => Err(ApiError::from(syscall.result)),
    }
}
*/

