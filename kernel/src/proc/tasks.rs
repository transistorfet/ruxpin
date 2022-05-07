
use alloc::string::String;

use ruxpin_api::types::{Tid, Pid, UserID};
use ruxpin_api::syscalls::{SyscallRequest, SyscallFunction};

use crate::arch::Context;
use crate::fs::filedesc::{FileDescriptors, SharableFileDescriptors};
use crate::mm::vmalloc::{VirtualAddressSpace, SharableVirtualAddressSpace};
use crate::sync::Spinlock;

use super::scheduler::Task;

const INIT_PID: Pid = 1;

static NEXT_TID: Spinlock<Tid> = Spinlock::new(1);

fn next_tid() -> Tid {
    let mut mutex = NEXT_TID.try_lock().unwrap();
    let tid = *mutex;
    *mutex += 1;
    tid
}

pub struct TaskCloneArgs {
    // TODO this is for the arguments telling what resources to clone
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
    pub tid: Tid,
    pub pid: Pid,

    // Process Data, Shared Amongs Threads
    pub parent: Pid,
    pub pgid: Pid,
    pub session: Pid,
    pub cmd: String,
    pub exit_status: Option<isize>,
    pub current_uid: UserID,

    // Other Module's Data
    pub space: SharableVirtualAddressSpace,
    pub files: SharableFileDescriptors,

    // Thread-Specific
    pub state: TaskState,
    pub syscall: SyscallRequest,
    pub restart_syscall: bool,
    pub context: Context,
}

impl TaskRecord {
    pub(super) fn new(parent: Option<Task>) -> Self {
        let tid = next_tid();

        let pid = tid;
        let (parent, pgid, session) = match parent {
            Some(parent_proc) => {
                let locked = parent_proc.try_lock().unwrap();
                (locked.pid, locked.pgid, locked.session)
            },
            None => (INIT_PID, pid, pid),
        };

        Self {
            tid,
            pid,
            parent,
            pgid,
            session,

            cmd: String::new(),
            exit_status: None,
            current_uid: 0,
            space: VirtualAddressSpace::new_sharable_user_space(),
            files: FileDescriptors::new_sharable(),

            state: TaskState::Running,
            syscall: Default::default(),
            restart_syscall: false,
            context: Default::default(),
        }
    }

    pub fn exit_and_free_resources(&mut self, status: isize) {
        self.exit_status = Some(status);
        self.free_resources();
    }

    pub fn free_resources(&mut self) {
        self.files.try_lock().unwrap().close_all();
        self.space.try_lock().unwrap().clear_segments();
    }

    pub fn clone_resources(&mut self, source: &TaskRecord, args: TaskCloneArgs) {
        self.current_uid = source.current_uid;
        self.files = source.files.try_lock().unwrap().duplicate_table();
        self.space.try_lock().unwrap().copy_segments(&source.space.try_lock().unwrap());
        self.context = source.context.clone();
        let ttbr = self.space.try_lock().unwrap().get_ttbr();
        self.context.set_ttbr(ttbr);

        // The return result will be 0 to indicate it's the child process
        self.context.write_result(Ok(0));
    }
}


