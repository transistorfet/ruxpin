
use alloc::vec::Vec;

use ruxpin_api::types::{Pid, UserID};
use ruxpin_api::syscalls::{SyscallRequest, SyscallFunction};

use crate::api::process_syscall;
use crate::arch::Context;
use crate::arch::types::VirtualAddress;
use crate::errors::KernelError;
use crate::fs::filedesc::FileDescriptors;
use crate::misc::queue::{Queue, QueueNode, QueueNodeRef};
use crate::mm::MemoryPermissions;
use crate::mm::vmalloc::VirtualAddressSpace;
use crate::sync::Spinlock;


pub struct ProcessRecord {
    // TODO I don't like that these are all pub... I might need to either isolate this more or change how things interact with processes
    pub pid: Pid,
    pub exit_status: Option<isize>,
    pub current_uid: UserID,
    pub space: VirtualAddressSpace,
    pub files: FileDescriptors,
    pub syscall: SyscallRequest,
    pub restart_syscall: bool,
    pub context: Context,
}

pub type Process = QueueNodeRef<ProcessRecord>;

struct ProcessManager {
    processes: Vec<QueueNodeRef<ProcessRecord>>,
    scheduled: Queue<ProcessRecord>,
    blocked: Queue<ProcessRecord>,
}

static NEXT_PID: Spinlock<Pid> = Spinlock::new(1);
static PROCESS_MANAGER: Spinlock<ProcessManager> = Spinlock::new(ProcessManager::new());


pub fn initialize() -> Result<(), KernelError> {
    //let idle = PROCESS_MANAGER.lock().create_process();
    //load_code(idle.cast(), TEST_PROC1);

    create_test_process();

    // NOTE this ensures the context is set before we start multitasking
    PROCESS_MANAGER.lock().schedule();
    Ok(())
}

impl ProcessManager {
    const fn new() -> Self {
        Self {
            processes: Vec::new(),
            scheduled: Queue::new(None),
            blocked: Queue::new(None),
        }
    }

    pub fn create_process(&mut self) -> Process {
        let pid = next_pid();

        self.processes.push(QueueNode::new(ProcessRecord {
            pid,
            exit_status: None,
            current_uid: 0,
            space: VirtualAddressSpace::new_user_space(),
            files: FileDescriptors::new(),
            syscall: Default::default(),
            restart_syscall: false,
            context: Default::default(),
        }));

        let current = self.processes.len() - 1;

        self.scheduled.insert_tail(self.processes[current].clone());

        self.processes[current].clone()
    }

    fn create_test_process(&mut self) -> Process {
        let proc = self.create_process();

        {
            let mut locked_proc = proc.try_lock().unwrap();

            // Allocate text segment
            locked_proc.space.add_memory_segment_allocated(MemoryPermissions::ReadExecute, VirtualAddress::from(0x77777000), 4096);

            // Allocate stack segment
            locked_proc.space.add_memory_segment(MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);

            let ttrb = locked_proc.space.get_ttbr();
            locked_proc.context.init(VirtualAddress::from(0x77777000), VirtualAddress::from(0x1_0000_0000), ttrb);
        }

        proc
    }

    pub fn get_current_process(&mut self) -> Process {
        if self.scheduled.get_head().is_none() {
            panic!("no scheduled processes when looking for the current process");
        }

        self.scheduled.get_head().unwrap().clone()
    }

    pub fn fork_current_process(&mut self) -> Process {
        let current_proc = self.scheduled.get_head().unwrap();
        let new_proc = self.create_process();

        new_proc.try_lock().unwrap().copy_resources(&*current_proc.try_lock().unwrap());

        new_proc
    }

    fn set_current_context(&mut self) -> Process {
        let new_current = self.get_current_process();
        Context::switch_current_context(&mut new_current.try_lock().unwrap().context);
        new_current
    }

    fn schedule(&mut self) -> Process {
        let current = self.scheduled.get_head().unwrap();

        self.scheduled.remove_node(current.clone());
        self.scheduled.insert_tail(current);

        self.set_current_context()
    }

    fn suspend(&mut self) {
        // also take an "event" arg
        // TODO actually you need an event to block on, and you could just put it in the process, but you... I supposed... could just
        // allocate the list nodes as needed and not have a vec (would mean a bunch of boxes, but that's ok... but also kinda implies the alloc::linked_list impl

        let current = self.scheduled.get_head().unwrap();

        self.scheduled.remove_node(current.clone());
        self.blocked.insert_head(current);

        self.set_current_context();
    }

    fn restart_blocked_by_syscall(&mut self, function: SyscallFunction) {
        for node in self.blocked.iter() {
            if node.try_lock().unwrap().syscall.function == function {
                self.blocked.remove_node(node.clone());
                self.scheduled.insert_head(node.clone());
                node.lock().restart_syscall = true;
            }
        }

        self.set_current_context();
    }

    fn exit_current_process(&mut self, status: isize) {
        let current = self.get_current_process();
        crate::printkln!("Exiting process {}", current.try_lock().unwrap().pid);

        self.scheduled.remove_node(current.clone());

        current.try_lock().unwrap().free_resources();
        current.try_lock().unwrap().exit_status = Some(status);

        self.restart_blocked_by_syscall(SyscallFunction::WaitPid);
    }

    fn page_fault(&mut self, far: u64) {
        self.get_current_process().try_lock().unwrap().space.alloc_page_at(VirtualAddress::from(far)).unwrap();
    }
}

impl ProcessRecord {
    pub fn free_resources(&mut self) {
        self.files.close_all();
        self.space.clear_segments();
    }

    pub fn copy_resources(&mut self, source: &ProcessRecord) {
        self.current_uid = source.current_uid;
        self.files = source.files.clone();
        self.space.copy_segments(&source.space);
        self.context = source.context.clone();
        let ttbr = self.space.get_ttbr();
        self.context.set_ttbr(ttbr);

        // The return result will be 0 to indicate it's the child process
        self.context.write_result(Ok(0));
    }
}

fn next_pid() -> Pid {
    let mut mutex = NEXT_PID.try_lock().unwrap();
    let pid = *mutex;
    *mutex += 1;
    pid
}

const TEST_PROC1: &[u32] = &[0xd503205f, 0x17ffffff];
//const TEST_PROC1: &[u32] = &[0xd40000e1, 0xd503205f, 0x17ffffff];
//const TEST_PROC2: &[u32] = &[0xd10043ff, 0xf90003e0, 0xf90007e1, 0x14000001, 0xd4000021, 0x17ffffff];

pub unsafe fn load_code(proc: Process, instructions: &[u32]) {
    let code: *mut u32 = proc.try_lock().unwrap().space.translate_addr(VirtualAddress::from(0x77777000)).unwrap().to_kernel_addr().as_mut();
    for i in 0..instructions.len() {
        *code.add(i) = instructions[i];
    }
}

pub fn create_test_process() {
    unsafe {
        let ptr = PROCESS_MANAGER.try_lock().unwrap().create_test_process();
        load_code(ptr, TEST_PROC1);

        //let ptr = PROCESS_MANAGER.lock().create_test_process();
        //load_code(ptr, TEST_PROC2);
    }
}


pub fn create_process() -> Process {
    PROCESS_MANAGER.try_lock().unwrap().create_process()
}

pub fn get_current_process() -> Process {
    PROCESS_MANAGER.try_lock().unwrap().get_current_process()
}

pub fn fork_current_process() -> Process {
    PROCESS_MANAGER.try_lock().unwrap().fork_current_process()
}

pub fn exit_current_process(status: isize) {
    PROCESS_MANAGER.try_lock().unwrap().exit_current_process(status)
}

pub(crate) fn schedule() {
    let new_current = PROCESS_MANAGER.try_lock().unwrap().schedule();

    if new_current.lock().restart_syscall {
        new_current.lock().restart_syscall = false;
        let mut syscall = new_current.lock().syscall.clone();
        process_syscall(&mut syscall);
    }
}

pub(crate) fn suspend_current_process() {
    PROCESS_MANAGER.try_lock().unwrap().suspend();
}

pub(crate) fn restart_blocked(function: SyscallFunction) {
    PROCESS_MANAGER.try_lock().unwrap().restart_blocked_by_syscall(function);
}

pub(crate) fn page_fault_handler(far: u64) {
    PROCESS_MANAGER.try_lock().unwrap().page_fault(far);
}

