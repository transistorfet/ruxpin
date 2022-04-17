
use ruxpin_api::types::{Pid, OpenFlags, FileAccess};

use crate::fs::vfs;
use crate::misc::StrArray;
use crate::errors::KernelError;
use crate::proc::process::{get_current_process, fork_current_process, exit_current_process, suspend_current_process};
use crate::proc::binaries::elf::loader;


pub fn syscall_exit(status: isize) -> Result<(), KernelError> {
    exit_current_process(status);
    Ok(())
}

pub fn syscall_fork() -> Result<Pid, KernelError> {
    let new_proc = fork_current_process();
    let child_pid = new_proc.lock().pid;
    Ok(child_pid)
}

pub fn syscall_exec(path: &str /*, _args: &[&str], _evnp: &[&str] */) -> Result<(), KernelError> {
    let proc = get_current_process();

    // Need to copy the path out of user memory before we free it all, but this should eventually use a copy_from_user() function
    let mut saved_path: StrArray<100> = StrArray::new();
    saved_path.copy_into(path);

    // TODO this causes a spinlock timeout
    //let cwd = proc.lock().files.get_cwd();
    //let current_uid = proc.lock().current_uid;
    //vfs::access(cwd, path, FileAccess::Exec.plus(FileAccess::Regular), current_uid)?;

    crate::printkln!("clearing old process space");
    proc.lock().free_resources();

    crate::printkln!("executing a new process");
    let result = loader::load_binary(proc.clone(), saved_path.as_str()).and_then(|_|
        proc.lock().files.open(None, "/dev/console0", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0)
    );

    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            exit_current_process(-1);
            Err(err)
        },
    }
}

pub fn syscall_waitpid(pid: Pid, status: &mut usize, _options: usize) -> Result<Pid, KernelError> {
    //let new_proc = fork_current_process();
    //let child_pid = new_proc.lock().pid;
    // TODO need to give a reason, an event
    // TODO this is so super hacky, but it'll work for now.  We just need to allow the process to restart if *any* process exits
    if *status != 0xdeadbeef {
        suspend_current_process();
        *status = 0xdeadbeef;
    }

    // TODO this should return the pid of the process that just exited, and also needs to use a proper status
    let pid = 1;
    Ok(pid)
}

