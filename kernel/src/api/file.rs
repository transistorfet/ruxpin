
use ruxpin_api::types::FileDesc;

use crate::errors::KernelError;
use crate::proc::process::get_current_proc;


pub fn syscall_write(file: FileDesc, buffer: &[u8]) -> Result<usize, KernelError> {
    let proc = get_current_proc();
    let mut locked_proc = proc.lock();
    locked_proc.files.write(file, buffer)
}

