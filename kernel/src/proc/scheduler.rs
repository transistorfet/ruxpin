
use alloc::vec::Vec;

use ruxpin_types::{Tid, Pid};
use ruxpin_syscall::{SyscallFunction};

use crate::api;
use crate::info;
use crate::arch::Context;
use crate::arch::types::VirtualAddress;
use crate::errors::KernelError;
use crate::misc::queue::{Queue, QueueNode, QueueNodeRef};
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
    TASK_MANAGER.lock().create_kernel_task("idle", idle_task)?;

    // NOTE this ensures the context is set before we start multitasking
    TASK_MANAGER.lock().schedule();
    Ok(())
}

fn idle_task() {
    loop {}
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

    fn create_kernel_task(&mut self, name: &str, entry: fn()) -> Result<(), KernelError> {
        let task = QueueNode::new(TaskRecord::initial_kernel_task(name));
        self.tasks.push(task.clone());
        self.scheduled.insert_tail(task.clone());

        let mut locked_task = task.try_lock()?;
        let ttbr = locked_task.space.try_lock()?.get_ttbr();
        locked_task.context.init_kernel_context(entry, VirtualAddress::from(0), ttbr);
        Ok(())
    }


    pub fn get_task(&mut self, tid: Tid) -> Option<Task> {
        for task in self.tasks.iter() {
            if task.try_lock().unwrap().task_id == tid {
                return Some(task.clone());
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

    fn suspend(&mut self, task: Task) {
        // also take an "event" arg
        // TODO actually you need an event to block on, and you could just put it in the process, but you... I supposed... could just
        // allocate the list nodes as needed and not have a vec (would mean a bunch of boxes, but that's ok... but also kinda implies the alloc::linked_list impl

        if task.try_lock().unwrap().state == TaskState::Running {
            task.try_lock().unwrap().state = TaskState::Blocked;
            self.scheduled.remove_node(task.clone());
            self.blocked.insert_head(task.clone());
        }

        self.set_current_context();
    }

    fn restart_blocked_by_syscall(&mut self, function: SyscallFunction) {
        for task in self.blocked.iter() {
            if task.try_lock().unwrap().syscall.function == function {
                if task.try_lock().unwrap().state == TaskState::Blocked {
                    task.try_lock().unwrap().state = TaskState::Running;
                    self.blocked.remove_node(task.clone());
                    self.scheduled.insert_head(task.clone());
                    task.try_lock().unwrap().restart_syscall = true;
                }
            }
        }

        self.set_current_context();
    }

    fn detach(&mut self, task: Task) {
        if task.try_lock().unwrap().state != TaskState::Exited {
            task.try_lock().unwrap().state = TaskState::Exited;
            self.scheduled.remove_node(task.clone());
        }

        self.set_current_context();
    }

    fn abort(&mut self, task: Task) {
        self.exit(task, -1);
    }

    fn exit(&mut self, task: Task, status: isize) {
        info!("Exiting process {}", task.try_lock().unwrap().process_id);

        self.detach(task.clone());
        let _ = task.try_lock().unwrap().exit_and_free_resources(status); // Ignore the error
        self.restart_blocked_by_syscall(SyscallFunction::WaitPid);
    }

    fn find_exited(&mut self, pid: Option<Pid>, parent: Option<Pid>, process_group: Option<Pid>) -> Option<Task> {
        for task in self.tasks.iter() {
            let locked_task = task.try_lock().unwrap();
            if
                locked_task.exit_status.is_some()
                && (pid.is_none() || locked_task.process_id == pid.unwrap())
                && (parent.is_none() || locked_task.parent_id == parent.unwrap())
                && (process_group.is_none() || locked_task.process_group_id == process_group.unwrap())
            {
                return Some(task.clone());
            }
        }

        None
    }

    fn clean_up(&mut self, pid: Pid) -> Result<(), KernelError> {
        for (i, task) in self.tasks.iter().enumerate() {
            if task.try_lock().unwrap().process_id == pid {
                if task.try_lock().unwrap().state != TaskState::Exited {
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

pub fn clone_current(args: TaskCloneArgs) -> Result<Task, KernelError> {
    let mut manager = TASK_MANAGER.try_lock()?;
    let current_task = manager.get_current();
    let new_proc = manager.create_task(Some(current_task.clone()));

    new_proc.try_lock()?.clone_resources(&*current_task.try_lock()?, args)?;

    Ok(new_proc)
}

pub fn abort(task: Task) {
    TASK_MANAGER.try_lock().unwrap().abort(task)
}

pub fn exit_current(status: isize) {
    let current_task = get_current();
    TASK_MANAGER.try_lock().unwrap().exit(current_task, status)
}

pub fn find_exited(pid: Option<Pid>, parent: Option<Pid>, process_group: Option<Pid>) -> Option<Task> {
    TASK_MANAGER.try_lock().unwrap().find_exited(pid, parent, process_group)
}

pub fn schedule() {
    TASK_MANAGER.try_lock().unwrap().schedule();
}

pub fn check_restart_syscall() {
    let current_task = get_current();
    if current_task.lock().restart_syscall {
        current_task.lock().restart_syscall = false;
        let mut syscall = current_task.lock().syscall.clone();
        api::process_syscall(&mut syscall);
    }
}

pub(crate) fn suspend(proc: Task) {
    TASK_MANAGER.try_lock().unwrap().suspend(proc);
}

pub(crate) fn restart_blocked(function: SyscallFunction) {
    TASK_MANAGER.try_lock().unwrap().restart_blocked_by_syscall(function);
}

