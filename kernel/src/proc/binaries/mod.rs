
pub mod elf;

use ruxpin_types::{OpenFlags, FileAccess};

use crate::errors::KernelError;
use crate::proc::binaries::elf::loader;
use crate::proc::scheduler::create_task;
use crate::misc::strarray::StandardArrayOfStrings;

pub fn load_process(cmd: &str) -> Result<(), KernelError> {
    let proc = create_task(None);
    let parsed_argv = StandardArrayOfStrings::new();
    let parsed_envp = StandardArrayOfStrings::new();
    loader::load_binary(proc.clone(), cmd, &parsed_argv, &parsed_envp)?;
    proc.lock().files.try_lock()?.open(None, "/dev/console0", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0)?;
    Ok(())
}

