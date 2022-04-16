
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::{Pid, UserID};
use ruxpin_api::syscalls::{SyscallRequest, SyscallFunction};

use crate::api::handle_syscall;
use crate::arch::Context;
use crate::arch::types::VirtualAddress;
use crate::fs::filedesc::FileDescriptors;
use crate::misc::linkedlist::{UnownedLinkedList, UnownedLinkedListNode};
use crate::mm::MemoryPermissions;
use crate::mm::vmalloc::VirtualAddressSpace;
use crate::sync::Spinlock;


pub struct ProcessRecord {
    // TODO I don't like that these are all pub... I might need to either isolate this more or change how things interact with processes
    pub pid: Pid,
    pub current_uid: UserID,
    pub space: VirtualAddressSpace,
    pub files: FileDescriptors,
    pub syscall: SyscallRequest,
    pub restart_syscall: bool,
    pub context: Context,
}

pub type Process = Arc<Spinlock<ProcessRecord>>;
pub struct Process2(Arc<Spinlock<UnownedLinkedListNode<ProcessRecord>>>);

struct ProcessManager {
// TODO should you make this Vec<UnownedLinkedListNode<Option<Process>>> so that you can close and reuse process entries to avoid changing their locations?
// TODO the alternative actually would be to get rid of the Vec, but also create a new queue type that *does* own the entries
    processes: Vec<UnownedLinkedListNode<Process>>,
    scheduled: UnownedLinkedList<Process>,
    blocked: UnownedLinkedList<Process>,
}

static NEXT_PID: Spinlock<Pid> = Spinlock::new(1);
static PROCESS_MANAGER: Spinlock<ProcessManager> = Spinlock::new(ProcessManager::new());


pub fn init_processes() {
    //let idle = PROCESS_MANAGER.lock().create_process();
    //load_code(idle.cast(), TEST_PROC1);

    create_test_process();

    // NOTE this ensures the context is set before we start multitasking
    PROCESS_MANAGER.lock().schedule();
}

impl ProcessManager {
    const fn new() -> Self {
        Self {
            processes: Vec::new(),
            scheduled: UnownedLinkedList::new(),
            blocked: UnownedLinkedList::new(),
        }
    }

    pub fn create_process(&mut self) -> Process {
        let pid = next_pid();

        self.processes.push(UnownedLinkedListNode::new(Arc::new(Spinlock::new(ProcessRecord {
            pid,
            current_uid: 0,
            space: VirtualAddressSpace::new_user_space(),
            files: FileDescriptors::new(),
            syscall: Default::default(),
            restart_syscall: false,
            context: Default::default(),
        }))));

        let current = self.processes.len() - 1;

        unsafe {
            self.scheduled.insert_tail(self.processes[current].as_node_ptr());
        }

        self.processes[current].clone()
    }

    fn create_test_process(&mut self) -> Process {
        let proc = self.create_process();

        {
            let mut locked_proc = proc.lock();

            // Allocate text segment
            //locked_proc.space.alloc_mapped(MemoryPermissions::ReadExecute, VirtualAddress::from(0x77777000), 4096);
            locked_proc.space.add_memory_segment_allocated(MemoryPermissions::ReadExecute, VirtualAddress::from(0x77777000), 4096);

            // Allocate stack segment
            //proc.space.alloc_mapped(MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);
            //locked_proc.space.map_on_demand(MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);
            locked_proc.space.add_memory_segment(MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);

            let ttrb = locked_proc.space.get_ttbr();
            locked_proc.context.init(VirtualAddress::from(0x77777000), VirtualAddress::from(0x1_0000_0000), ttrb);
        }

        proc
    }

    /*
    // TODO this is temporary to suppress unused warnings
    #[allow(dead_code)]
    fn destroy_process(&mut self, proc: &mut ProcessRecord) {
        proc.space.unmap_range(VirtualAddress::from(0), usize::MAX);
    }
    */

    pub fn get_current_process(&mut self) -> Process {
        if self.scheduled.get_head().is_none() {
            panic!("no scheduled processes when looking for the current process");
        }

        unsafe {
            self.scheduled.get_head().unwrap().get().clone()
        }
    }

    pub fn fork_current_process(&mut self) -> Process {
        let current_proc = self.scheduled.get_head().unwrap();
        let new_proc = self.create_process();

        {
            let locked_current_proc = unsafe { current_proc.get() }.lock();
            let mut locked_new_proc = new_proc.lock();
            locked_new_proc.current_uid = locked_current_proc.current_uid;
            locked_new_proc.files = locked_current_proc.files.clone();
            locked_new_proc.space.copy_segments(&locked_current_proc.space);
            locked_new_proc.context = locked_current_proc.context.clone();
            let ttbr = locked_new_proc.space.get_ttbr();
            locked_new_proc.context.set_ttbr(ttbr);

            // The return result will be 0 to indicate it's the child process
            locked_new_proc.context.write_result(Ok(0));
        }

        new_proc
    }


    fn schedule(&mut self) -> Process {
        let current = self.scheduled.get_head().unwrap();

        unsafe {
            self.scheduled.remove_node(current);
            self.scheduled.insert_tail(current);
        }

        let new_current = self.get_current_process();
        Context::switch_current_context(&mut new_current.lock().context);

        new_current
    }

    fn restart_blocked_by_syscall(&mut self, function: SyscallFunction) {
        for node in self.blocked.iter() {
            if unsafe { node.get() }.lock().syscall.function == function {
                unsafe {
                    self.blocked.remove_node(node);
                    self.scheduled.insert_head(node);
                }
                unsafe { node.get() }.lock().restart_syscall = true;
            }
        }
    }

    fn suspend(&mut self) {
        // also take an "event" arg
        // TODO actually you need an event to block on, and you could just put it in the process, but you... I supposed... could just
        // allocate the list nodes as needed and not have a vec (would mean a bunch of boxes, but that's ok... but also kinda implies the alloc::linked_list impl

        let current = self.scheduled.get_head().unwrap();

        unsafe {
            self.scheduled.remove_node(current);
            self.blocked.insert_head(current);
        }

        let new_current = self.get_current_process();
        Context::switch_current_context(&mut new_current.lock().context);
    }

    fn exit_current_process(&mut self, status: usize) {
        let current = self.scheduled.get_head().unwrap();
        crate::printkln!("Exiting process {}", unsafe { current.get() }.lock().pid);

        // TODO this is just junking the process record (it will never be removed but never be scheduled again)
        //      It shouldn't be removed here anyways, but it would be when the parent proc "wait()"s for it
        unsafe {
            self.scheduled.remove_node(current);
        }

        let new_current = self.get_current_process();
        Context::switch_current_context(&mut new_current.lock().context);

        self.restart_blocked_by_syscall(SyscallFunction::WaitPid);
    }

    fn page_fault(&mut self, far: u64) {
        self.get_current_process().lock().space.alloc_page_at(VirtualAddress::from(far)).unwrap();
    }
}

fn next_pid() -> Pid {
    let mut mutex = NEXT_PID.lock();
    let pid = *mutex;
    *mutex += 1;
    pid
}

const TEST_PROC1: &[u32] = &[0xd503205f, 0x17ffffff];
//const TEST_PROC1: &[u32] = &[0xd40000e1, 0xd503205f, 0x17ffffff];
//const TEST_PROC2: &[u32] = &[0xd10043ff, 0xf90003e0, 0xf90007e1, 0x14000001, 0xd4000021, 0x17ffffff];

pub unsafe fn load_code(proc: Process, instructions: &[u32]) {
    let code: *mut u32 = proc.lock().space.translate_addr(VirtualAddress::from(0x77777000)).unwrap().to_kernel_addr().as_mut();
    for i in 0..instructions.len() {
        *code.add(i) = instructions[i];
    }
}

pub fn create_test_process() {
    unsafe {
        let ptr = PROCESS_MANAGER.lock().create_test_process();
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

pub fn exit_current_process(status: usize) {
    PROCESS_MANAGER.try_lock().unwrap().exit_current_process(status)
}

pub(crate) fn schedule() {
    let new_current = PROCESS_MANAGER.try_lock().unwrap().schedule();

    if new_current.lock().restart_syscall {
        new_current.lock().restart_syscall = false;
        let mut syscall = new_current.lock().syscall.clone();
        handle_syscall(&mut syscall);
        Context::write_syscall_result_to_current_context(&syscall);
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

