
use ruxpin_api::types::FileDesc;

use crate::errors::KernelError;
use crate::proc::process::get_current_proc;

pub fn syscall_write(file: FileDesc, buffer: &[u8]) -> Result<usize, KernelError> {
    use crate::arch::types::VirtualAddress;
    crate::printkln!("In the write syscall!!! {:?} {:?} {:?}", file.0, buffer.as_ptr() as u64, buffer.len());

    let proc = get_current_proc();
    let mut locked_proc = proc.lock();
    let addr = locked_proc.space.translate_addr(VirtualAddress::from(buffer.as_ptr() as u64))?;
    unsafe { crate::printkln!("{:?}", *(addr.to_kernel_addr().as_ptr() as *const u8)); }
    locked_proc.files.write(file, buffer)
}

