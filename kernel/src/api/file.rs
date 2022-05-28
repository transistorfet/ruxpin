
use ruxpin_types::{FileDesc, OpenFlags, FileAccess, DirEntry, UserID};
use ruxpin_syscall_proc::syscall_handler;

use crate::fs::vfs;
use crate::fs::types::Vnode;
use crate::errors::KernelError;
use crate::proc::scheduler::get_current;


#[syscall_handler]
pub fn syscall_open(path: &str, flags: OpenFlags, access: FileAccess) -> Result<FileDesc, KernelError> {
    let proc = get_current();

    let (cwd, current_uid, file_num) = {
        let locked_proc = proc.try_lock()?;

        let cwd = locked_proc.files.try_lock()?.get_cwd();
        let current_uid = locked_proc.current_uid;
        let file_num = locked_proc.files.try_lock()?.find_free_slot()?;
        (cwd, current_uid, file_num)
    };

    let file = vfs::open(cwd, path, flags, access, current_uid)?;
    proc.try_lock()?.files.try_lock()?.set_slot(file_num, file)?;
    Ok(file_num)
}

#[syscall_handler]
pub fn syscall_close(file: FileDesc) -> Result<(), KernelError> {
    let proc = get_current();
    let result = proc.try_lock()?.files.try_lock()?.clear_slot(file);
    result
}

#[syscall_handler]
pub fn syscall_read(file: FileDesc, buffer: &mut [u8]) -> Result<usize, KernelError> {
    let file = get_current().try_lock()?.files.try_lock()?.get_file(file)?;
    vfs::read(file, buffer)
}

#[syscall_handler]
pub fn syscall_write(file: FileDesc, buffer: &[u8]) -> Result<usize, KernelError> {
    let file = get_current().try_lock()?.files.try_lock()?.get_file(file)?;
    vfs::write(file, buffer)
}

#[syscall_handler]
pub fn syscall_readdir(file: FileDesc, dirent: &mut DirEntry) -> Result<bool, KernelError> {
    let file = get_current().try_lock()?.files.try_lock()?.get_file(file)?;
    match vfs::readdir(file)? {
        Some(result) => {
            *dirent = result;
            Ok(true)
        },
        None => Ok(false),
    }
}

#[syscall_handler]
pub fn syscall_unlink(path: &str) -> Result<(), KernelError> {
    let (cwd, current_uid) = get_current_cwd_and_uid()?;
    vfs::unlink(cwd, path, current_uid)?;
    Ok(())
}

#[syscall_handler]
pub fn syscall_rename(old_path: &str, new_path: &str) -> Result<(), KernelError> {
    let (cwd, current_uid) = get_current_cwd_and_uid()?;
    vfs::rename(cwd, old_path, new_path, current_uid)?;
    Ok(())
}

#[syscall_handler]
pub fn syscall_mkdir(path: &str, access: FileAccess) -> Result<(), KernelError> {
    let (cwd, current_uid) = get_current_cwd_and_uid()?;
    vfs::make_directory(cwd, path, access, current_uid)?;
    Ok(())
}

#[syscall_handler]
pub fn syscall_getcwd(path: &mut [u8]) -> Result<(), KernelError> {

    Err(KernelError::OperationNotPermitted)
}

fn get_current_cwd_and_uid() -> Result<(Option<Vnode>, UserID), KernelError> {
    let proc = get_current();
    let locked_proc = proc.try_lock()?;

    let cwd = locked_proc.files.try_lock()?.get_cwd();
    let current_uid = locked_proc.current_uid;
    Ok((cwd, current_uid))
}

