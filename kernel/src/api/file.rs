
use ruxpin_api::types::{FileDesc, OpenFlags, FileAccess, DirEntry};

use crate::fs::vfs;
use crate::errors::KernelError;
use crate::proc::scheduler::get_current;


pub fn syscall_open(path: &str, flags: OpenFlags, access: FileAccess) -> Result<FileDesc, KernelError> {
    let proc = get_current();

    let (cwd, current_uid, file_num) = {
        let locked_proc = proc.try_lock().unwrap();

        let cwd = locked_proc.files.try_lock().unwrap().get_cwd();
        let current_uid = locked_proc.current_uid;
        let file_num = locked_proc.files.try_lock().unwrap().find_free_slot()?;
        (cwd, current_uid, file_num)
    };

    let file = vfs::open(cwd, path, flags, access, current_uid)?;
    proc.try_lock().unwrap().files.try_lock().unwrap().set_slot(file_num, file)?;
    Ok(file_num)
}

pub fn syscall_close(file: FileDesc) -> Result<(), KernelError> {
    let proc = get_current();
    let result = proc.try_lock().unwrap().files.try_lock().unwrap().clear_slot(file);
    result
}

pub fn syscall_read(file: FileDesc, buffer: &mut [u8]) -> Result<usize, KernelError> {
    let file = get_current().try_lock().unwrap().files.try_lock().unwrap().get_file(file)?;
    vfs::read(file, buffer)
}

pub fn syscall_write(file: FileDesc, buffer: &[u8]) -> Result<usize, KernelError> {
    let file = get_current().try_lock().unwrap().files.try_lock().unwrap().get_file(file)?;
    vfs::write(file, buffer)
}

pub fn syscall_readdir(file: FileDesc, dirent: &mut DirEntry) -> Result<bool, KernelError> {
    let file = get_current().try_lock().unwrap().files.try_lock().unwrap().get_file(file)?;
    match vfs::readdir(file)? {
        Some(result) => {
            *dirent = result;
            Ok(true)
        },
        None => Ok(false),
    }
}

