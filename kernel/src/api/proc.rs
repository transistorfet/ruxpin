
use ruxpin_types::Pid;
use ruxpin_syscall_proc::syscall_handler;

use crate::proc::scheduler;
use crate::errors::KernelError;
use crate::proc::scheduler::Task;
use crate::proc::tasks::TaskCloneArgs;
use crate::misc::strarray::{StrArray, StandardArrayOfStrings};

use super::binaries::elf::loader;


#[syscall_handler]
pub fn syscall_exit(status: isize) -> Result<(), KernelError> {
    scheduler::exit_current(status);
    Ok(())
}

#[syscall_handler]
pub fn syscall_fork() -> Result<Pid, KernelError> {
    let args = TaskCloneArgs::new();
    let new_proc = scheduler::clone_current(args)?;
    let child_pid = new_proc.try_lock()?.process_id;
    Ok(child_pid)
}

#[syscall_handler]
pub fn syscall_exec(path: &str, argv: &[&str], envp: &[&str]) -> Result<(), KernelError> {
    // This function must not return an error without exiting the process
    let proc = scheduler::get_current();

    let parsed_argv = StandardArrayOfStrings::new_parsed(argv);
    let parsed_envp = StandardArrayOfStrings::new_parsed(envp);

    // Need to copy the path out of user memory before we free it all, but this should eventually use a copy_from_user() function
    let mut saved_path: StrArray<100> = StrArray::new();
    saved_path.copy_into(path);

    let result = setup_process(proc, saved_path.as_str(), &parsed_argv, &parsed_envp);
    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            scheduler::exit_current(-1);
            Err(err)
        },
    }
}

fn setup_process(proc: Task, path: &str, argv: &StandardArrayOfStrings, envp: &StandardArrayOfStrings) -> Result<(), KernelError> {
    // This function can return an error safely

    proc.try_lock()?.free_memory()?;

    loader::load_binary(proc.clone(), path, argv, envp)?;

    Ok(())
}

#[syscall_handler]
pub fn syscall_waitpid(pid: Pid, status: &mut isize, _options: usize) -> Result<Pid, KernelError> {
    let parent_id = scheduler::get_current().lock().process_id;

    let search_pid = if pid > 0 { Some(pid) } else { None };
    let search_parent = if pid == 0 { Some(parent_id) } else { None };
    let proc = scheduler::find_exited(search_pid, search_parent, None);

    if let Some(proc) = proc {
        let pid = proc.try_lock()?.process_id;
        if status as *mut isize as usize != 0 {
            *status = proc.try_lock()?.exit_status.unwrap();
        }
        scheduler::clean_up(pid)?;
        Ok(pid)
    } else {
        scheduler::suspend(scheduler::get_current());
        Ok(0)
    }
}

#[syscall_handler]
pub fn syscall_sbrk(increment: isize) -> Result<*const u8, KernelError> {
    let proc = scheduler::get_current();
    let old_break = proc.try_lock()?.space.try_lock()?.adjust_stack_break(increment)?;

    Ok(usize::from(old_break) as *const u8)
}

