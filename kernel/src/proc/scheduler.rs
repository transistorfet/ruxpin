
use alloc::vec::Vec;

use ruxpin_api::types::{Tid, Pid};
use ruxpin_syscall::{SyscallFunction};

use crate::api::process_syscall;
use crate::arch::Context;
use crate::arch::types::VirtualAddress;
use crate::errors::KernelError;
use crate::misc::queue::{Queue, QueueNode, QueueNodeRef};
use crate::mm::MemoryPermissions;
use crate::mm::segments::SegmentType;
use crate::sync::Spinlock;

use super::tasks::{TaskCloneArgs, TaskState, TaskRecord};

pub type Task = QueueNodeRef<TaskRecord>;

struct TaskManager {
    tasks: Vec<QueueNodeRef<TaskRecord>>,
    scheduled: Queue<TaskRecord>,
    blocked: Queue<TaskRecord>,
}

static TASK_MANAGER: Spinlock<TaskManager> = Spinlock::new(TaskManager::new());

pub fn initialize() -> Result<(), KernelError> {
    //let idle = TASK_MANAGER.lock().create_task();
    //load_code(idle.cast(), TEST_PROC1);

    create_test_process();

    // NOTE this ensures the context is set before we start multitasking
    TASK_MANAGER.lock().schedule();
    Ok(())
}

impl TaskManager {
    const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            scheduled: Queue::new(None),
            blocked: Queue::new(None),
        }
    }

    pub fn create_task(&mut self, parent: Option<Task>) -> Task {
        let task = QueueNode::new(TaskRecord::new(parent));
        self.tasks.push(task.clone());
        self.scheduled.insert_tail(task.clone());
        task
    }

    pub fn get_task(&mut self, tid: Tid) -> Option<Task> {
        for proc in self.tasks.iter() {
            if proc.try_lock().unwrap().task_id == tid {
                return Some(proc.clone());
            }
        }
        None
    }

    pub fn get_process(&mut self, pid: Pid) -> Option<Task> {
        // The main thread will have tid == pid
        self.get_task(pid)
    }

    pub fn get_slot(&mut self, slot: usize) -> Option<Task> {
        if slot < self.tasks.len() {
            Some(self.tasks[slot].clone())
        } else {
            None
        }
    }

    pub fn slot_len(&mut self) -> usize {
        self.tasks.len()
    }

    pub fn get_current(&mut self) -> Task {
        if self.scheduled.get_head().is_none() {
            panic!("no scheduled tasks when looking for the current process");
        }

        self.scheduled.get_head().unwrap().clone()
    }

    fn set_current_context(&mut self) -> Task {
        let new_current = self.get_current();
        Context::switch_current_context(&mut new_current.try_lock().unwrap().context);
        new_current
    }

    fn schedule(&mut self) {
        let current = self.get_current();

        self.scheduled.remove_node(current.clone());
        self.scheduled.insert_tail(current);

        self.set_current_context();
    }

    fn suspend(&mut self, proc: Task) {
        // also take an "event" arg
        // TODO actually you need an event to block on, and you could just put it in the process, but you... I supposed... could just
        // allocate the list nodes as needed and not have a vec (would mean a bunch of boxes, but that's ok... but also kinda implies the alloc::linked_list impl

        if proc.try_lock().unwrap().state == TaskState::Running {
            proc.try_lock().unwrap().state = TaskState::Blocked;
            self.scheduled.remove_node(proc.clone());
            self.blocked.insert_head(proc.clone());
        }

        self.set_current_context();
    }

    fn restart_blocked_by_syscall(&mut self, function: SyscallFunction) {
        for node in self.blocked.iter() {
            if node.try_lock().unwrap().syscall.function == function {
                if node.try_lock().unwrap().state == TaskState::Blocked {
                    node.try_lock().unwrap().state = TaskState::Running;
                    self.blocked.remove_node(node.clone());
                    self.scheduled.insert_head(node.clone());
                    node.lock().restart_syscall = true;
                }
            }
        }

        self.set_current_context();
    }

    fn detach(&mut self, proc: Task) {
        if proc.try_lock().unwrap().state != TaskState::Exited {
            proc.try_lock().unwrap().state = TaskState::Exited;
            self.scheduled.remove_node(proc.clone());
        }
    }

    fn exit_current(&mut self, status: isize) {
        let current = self.get_current();
        crate::printkln!("Exiting process {}", current.try_lock().unwrap().process_id);

        self.detach(current.clone());
        current.try_lock().unwrap().exit_and_free_resources(status);
        self.restart_blocked_by_syscall(SyscallFunction::WaitPid);
    }

    fn find_exited(&mut self, pid: Option<Pid>, parent: Option<Pid>, process_group: Option<Pid>) -> Option<Task> {
        for process in self.tasks.iter() {
            let locked_proc = process.try_lock().unwrap();
            if
                locked_proc.exit_status.is_some()
                && (pid.is_none() || locked_proc.process_id == pid.unwrap())
                && (parent.is_none() || locked_proc.parent_id == parent.unwrap())
                && (process_group.is_none() || locked_proc.process_group_id == process_group.unwrap())
            {
                return Some(process.clone());
            }
        }

        None
    }

    fn clean_up(&mut self, pid: Pid) -> Result<(), KernelError> {
        for (i, process) in self.tasks.iter().enumerate() {
            if process.try_lock().unwrap().process_id == pid {
                if process.try_lock().unwrap().state != TaskState::Exited {
                    return Err(KernelError::NotExited);
                }
                self.tasks.remove(i);
                self.set_current_context();
                return Ok(());
            }
        }
        Err(KernelError::NoSuchTask)
    }
}

pub fn create_task(parent: Option<Task>) -> Task {
    TASK_MANAGER.try_lock().unwrap().create_task(parent)
}

pub fn clean_up(pid: Pid) -> Result<(), KernelError> {
    TASK_MANAGER.try_lock().unwrap().clean_up(pid)
}

pub fn get_task(tid: Tid) -> Option<Task> {
    TASK_MANAGER.try_lock().unwrap().get_task(tid)
}

pub fn get_process(pid: Pid) -> Option<Task> {
    TASK_MANAGER.try_lock().unwrap().get_process(pid)
}

pub fn get_slot(slot: usize) -> Option<Task> {
    TASK_MANAGER.try_lock().unwrap().get_slot(slot)
}

pub fn slot_len() -> usize {
    TASK_MANAGER.try_lock().unwrap().slot_len()
}

pub fn get_current() -> Task {
    TASK_MANAGER.try_lock().unwrap().get_current()
}

pub fn clone_current(args: TaskCloneArgs) -> Task {
    let mut manager = TASK_MANAGER.try_lock().unwrap();
    let current_proc = manager.get_current();
    let new_proc = manager.create_task(Some(current_proc.clone()));

    new_proc.try_lock().unwrap().clone_resources(&*current_proc.try_lock().unwrap(), args);

    new_proc
}

pub fn exit_current(status: isize) {
    TASK_MANAGER.try_lock().unwrap().exit_current(status)
}

pub fn find_exited(pid: Option<Pid>, parent: Option<Pid>, process_group: Option<Pid>) -> Option<Task> {
    TASK_MANAGER.try_lock().unwrap().find_exited(pid, parent, process_group)
}

pub fn schedule() {
    TASK_MANAGER.try_lock().unwrap().schedule();
    check_restart_syscall();
}

pub(crate) fn check_restart_syscall() {
    let current_proc = get_current();
    if current_proc.lock().restart_syscall {
        current_proc.lock().restart_syscall = false;
        let mut syscall = current_proc.lock().syscall.clone();
        process_syscall(&mut syscall);
    }
}

pub(crate) fn suspend(proc: Task) {
    TASK_MANAGER.try_lock().unwrap().suspend(proc);
}

pub(crate) fn restart_blocked(function: SyscallFunction) {
    TASK_MANAGER.try_lock().unwrap().restart_blocked_by_syscall(function);
}


// TODO this is aarch64 specific and will eventually be removed

const TEST_PROC1: &[u32] = &[0xd503205f, 0x17ffffff];
//const TEST_PROC1: &[u32] = &[0xd40000e1, 0xd503205f, 0x17ffffff];
//const TEST_PROC2: &[u32] = &[0xd10043ff, 0xf90003e0, 0xf90007e1, 0x14000001, 0xd4000021, 0x17ffffff];

pub unsafe fn load_code(proc: Task, instructions: &[u32]) {
    let code: *mut u32 = proc.try_lock().unwrap().space.try_lock().unwrap().translate_addr(VirtualAddress::from(0x77777000)).unwrap().to_kernel_addr().as_mut();
    for i in 0..instructions.len() {
        *code.add(i) = instructions[i];
    }
}

fn create_test_process() {
    unsafe {
        let proc = create_task(None);
        {
            let mut locked_proc = proc.try_lock().unwrap();

            let ttrb = {
                let mut space = locked_proc.space.try_lock().unwrap();

                // Allocate text segment
                space.add_memory_segment_allocated(SegmentType::Text, MemoryPermissions::ReadExecute, VirtualAddress::from(0x77777000), 4096);

                // Allocate stack segment
                space.add_memory_segment(SegmentType::Stack, MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);

                space.get_ttbr()
            };
            locked_proc.context.init(VirtualAddress::from(0x77777000), VirtualAddress::from(0x1_0000_0000), ttrb);
        }
        load_code(proc, TEST_PROC1);

        //let ptr = TASK_MANAGER.lock().create_test_process();
        //load_code(ptr, TEST_PROC2);
    }
}

