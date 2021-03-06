
use alloc::string::String;
use alloc::string::ToString;

use ruxpin_syscall::SyscallRequest;
use ruxpin_types::{Tid, Pid, UserID};

use crate::arch::Context;
use crate::sync::Spinlock;
use crate::errors::KernelError;
use crate::fs::{FileDescriptors, SharableFileDescriptors};
use crate::mm::{VirtualAddressSpace, SharableVirtualAddressSpace};

use super::scheduler::Task;

const INIT_PID: Pid = 1;

static NEXT_TID: Spinlock<Tid> = Spinlock::new(1);

fn next_task_id() -> Tid {
    let mut mutex = NEXT_TID.try_lock().unwrap();
    let task_id = *mutex;
    *mutex += 1;
    task_id
}

pub struct TaskCloneArgs {
    // TODO this is for the arguments telling what resources to clone
    //flags: TaskCloneFlags,
}

impl TaskCloneArgs {
    pub fn new() -> Self {
        Self {

        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskState {
    Exited,
    Running,
    Blocked,
}

// TODO I don't like that these are all pub... I might need to either isolate this more or change how things interact with tasks
pub struct TaskRecord {
    // Immutable Data
    pub task_id: Tid,
    pub process_id: Pid,

    // Process Data, Shared Amongs Threads
    pub parent_id: Pid,
    pub process_group_id: Pid,
    pub session_id: Pid,
    pub cmd: String,
    pub current_uid: UserID,

    // Other Module's Data
    pub space: SharableVirtualAddressSpace,
    pub files: SharableFileDescriptors,

    // Thread-Specific
    pub exit_status: Option<isize>,
    pub state: TaskState,
    pub syscall: SyscallRequest,
    pub restart_syscall: bool,
    pub context: Context,
}

impl TaskRecord {
    pub(super) fn initial_kernel_task(cmd: &str) -> Self {
        let task_id = next_task_id();

        Self {
            task_id,
            process_id: task_id,

            parent_id: task_id,
            process_group_id: task_id,
            session_id: task_id,
            cmd: cmd.to_string(),
            current_uid: 0,

            space: VirtualAddressSpace::get_kernel_space(),
            files: FileDescriptors::new_sharable(),

            exit_status: None,
            state: TaskState::Running,
            syscall: Default::default(),
            restart_syscall: false,
            context: Default::default(),
        }
    }

    pub(super) fn new(parent: Option<Task>) -> Self {
        let task_id = next_task_id();

        let process_id = task_id;
        let (parent_id, process_group_id, session_id) = match parent {
            Some(parent_proc) => {
                let locked = parent_proc.try_lock().unwrap();
                (locked.process_id, locked.process_group_id, locked.session_id)
            },
            None => (INIT_PID, process_id, process_id),
        };

        Self {
            task_id,
            process_id,

            parent_id,
            process_group_id,
            session_id,
            cmd: String::new(),
            current_uid: 0,

            space: VirtualAddressSpace::new_sharable(),
            files: FileDescriptors::new_sharable(),

            exit_status: None,
            state: TaskState::Running,
            syscall: Default::default(),
            restart_syscall: false,
            context: Default::default(),
        }
    }

    pub fn exit_and_free_resources(&mut self, status: isize) -> Result<(), KernelError> {
        self.exit_status = Some(status);
        self.free_resources()
    }

    pub fn free_resources(&mut self) -> Result<(), KernelError> {
        self.files.try_lock()?.close_all();
        self.space.try_lock()?.clear_segments()?;
        Ok(())
    }

    pub fn free_memory(&mut self) -> Result<(), KernelError> {
        self.space.try_lock()?.clear_segments()?;
        Ok(())
    }

    pub fn clone_resources(&mut self, source: &TaskRecord, _args: TaskCloneArgs) -> Result<(), KernelError> {
        self.current_uid = source.current_uid;
        self.files = source.files.try_lock()?.duplicate_table();
        self.space.try_lock()?.copy_segments(&mut *source.space.try_lock()?)?;
        let ttbr = self.space.try_lock()?.get_ttbr();
        self.context = source.context.clone();
        self.context.set_ttbr(ttbr);

        // The return result will be 0 to indicate it's the child process
        self.context.write_result(Ok(0));
        Ok(())
    }
}

