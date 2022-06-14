
pub mod elf;

use ruxpin_types::{FileDesc, OpenFlags, FileAccess};

use crate::fs::vfs;
use crate::errors::KernelError;
use crate::proc::scheduler::create_task;
use crate::misc::strarray::StandardArrayOfStrings;

use self::elf::loader;

pub fn load_process(cmd: &str) -> Result<(), KernelError> {
    let proc = create_task(None);
    let parsed_argv = StandardArrayOfStrings::new();
    let parsed_envp = StandardArrayOfStrings::new();
    loader::load_binary(proc.clone(), cmd, &parsed_argv, &parsed_envp)?;

    {
        let files = proc.lock().files.clone();
        let mut locked_files = files.try_lock()?;
        let file = vfs::open(None, "/dev/console0", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0)?;
        locked_files.set_slot(FileDesc(0), file.clone())?;
        locked_files.set_slot(FileDesc(1), file.clone())?;
        locked_files.set_slot(FileDesc(2), file)?;
    }

    Ok(())
}

