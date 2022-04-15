
use ruxpin_api::types::{OpenFlags, FileAccess};

use crate::misc::StrArray;
use crate::errors::KernelError;
use crate::proc::process::{get_current_proc, exit_current_proc};
use crate::proc::binaries::elf::loader;


pub fn syscall_exit(status: usize) -> Result<(), KernelError> {
    exit_current_proc(status);
    Ok(())
}

pub fn syscall_exec(path: &str /*, _args: &[&str], _evnp: &[&str] */) -> Result<(), KernelError> {
    let proc = get_current_proc();

    // Need to copy the path out of user memory before we free it all, but this should eventually use a copy_from_user() function
    let mut saved_path: StrArray<100> = StrArray::new();
    saved_path.copy_into(path);

    crate::printkln!("clearing old process space");
    {
        let mut locked_proc = proc.lock();
        locked_proc.files.close_all();
        locked_proc.space.clear_segments();
    }

    crate::printkln!("executing a new process");
    loader::load_binary(proc.clone(), saved_path.as_str()).unwrap();
    proc.lock().files.open(None, "/dev/console0", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0).unwrap();

    Ok(())
}


