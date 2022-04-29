
use ruxpin_api::types::{Pid, OpenFlags, FileAccess};

use crate::errors::KernelError;
use crate::misc::strarray::{StrArray, StandardArrayOfStrings};
use crate::proc::process::{get_current_process, fork_current_process, exit_current_process, find_exited_process, suspend_process, clean_up_process};
use crate::proc::binaries::elf::loader;


pub fn syscall_exit(status: isize) -> Result<(), KernelError> {
    exit_current_process(status);
    Ok(())
}

pub fn syscall_fork() -> Result<Pid, KernelError> {
    let new_proc = fork_current_process();
    let child_pid = new_proc.try_lock().unwrap().pid;
    Ok(child_pid)
}

pub fn syscall_exec(path: &str, argv: &[&str], envp: &[&str]) -> Result<(), KernelError> {
    let proc = get_current_process();

    let parsed_argv = StandardArrayOfStrings::new_parsed(argv);
    let parsed_envp = StandardArrayOfStrings::new_parsed(envp);

    // Need to copy the path out of user memory before we free it all, but this should eventually use a copy_from_user() function
    let mut saved_path: StrArray<100> = StrArray::new();
    saved_path.copy_into(path);

    // TODO this causes a spinlock timeout
    //let cwd = proc.lock().files.get_cwd();
    //let current_uid = proc.lock().current_uid;
    //vfs::access(cwd, path, FileAccess::Exec.plus(FileAccess::Regular), current_uid)?;

    crate::printkln!("clearing old process space");
    proc.try_lock().unwrap().free_resources();

    proc.try_lock().unwrap().files.open(None, "/dev/console0", OpenFlags::ReadWrite, FileAccess::DefaultFile, 0)?;

    crate::printkln!("executing a new process");
    let result = loader::load_binary(proc.clone(), saved_path.as_str(), &parsed_argv, &parsed_envp);

    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            exit_current_process(-1);
            Err(err)
        },
    }
}

pub fn syscall_waitpid(pid: Pid, status: &mut isize, _options: usize) -> Result<Pid, KernelError> {
    let parent_id = get_current_process().lock().pid;

    let search_pid = if pid > 0 { Some(pid) } else { None };
    let search_parent = if pid == 0 { Some(parent_id) } else { None };
    let proc = find_exited_process(search_pid, search_parent, None);

    if let Some(proc) = proc {
        let pid = proc.try_lock().unwrap().pid;
        *status = proc.try_lock().unwrap().exit_status.unwrap();
        crate::printkln!("cleaning up process {}", pid);
        clean_up_process(pid);
        Ok(pid)
    } else {
        suspend_process(get_current_process());
        Ok(0)
    }
}

