
use core::ptr;

use alloc::vec::Vec;

use ruxpin_api::types::UserID;

use crate::mm::MemoryPermissions;
use crate::mm::vmalloc::VirtualAddressSpace;

use crate::arch::Context;
use crate::sync::Spinlock;
use crate::arch::types::VirtualAddress;
use crate::fs::filedesc::FileDescriptors;

pub type Pid = i32;

pub struct Process {
    pid: Pid,
    uid: UserID,
    context: Context,
    space: VirtualAddressSpace,
    files: FileDescriptors,
}

struct ProcessManager {
    processes: Vec<Process>,
    current: usize,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}


static NEXT_PID: Spinlock<Pid> = Spinlock::new(1);
static PROCESS_MANAGER: Spinlock<ProcessManager> = Spinlock::new(ProcessManager::new());

// TODO need to move this
#[no_mangle]
pub static mut CURRENT_CONTEXT: *mut Context = ptr::null_mut();


pub fn init_processes() {
    //let idle = PROCESS_MANAGER.lock().create_process();
    //load_code(idle.cast(), TEST_PROC1);

    create_test_process();

    PROCESS_MANAGER.lock().schedule();
}

impl ProcessManager {
    const fn new() -> Self {
        Self {
            processes: Vec::new(),
            current: 0,
        }
    }

    fn create_process(&mut self) -> *mut u8 {
        let pid = next_pid();

        self.processes.push(Process {
            pid,
            uid: 0,
            context: Default::default(),
            space: VirtualAddressSpace::new_user_space(),
            files: FileDescriptors::new(),
        });

        self.current = self.processes.len() - 1;
        let proc = &mut self.processes[self.current];
        // Allocate text segment
        let entry = proc.space.alloc_mapped(MemoryPermissions::ReadExecute, VirtualAddress::from(0x77777000), 4096);
        // Allocate stack segment
        //proc.space.alloc_mapped(MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);
        proc.space.map_on_demand(MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);
        Context::init(&mut proc.context, VirtualAddress::from(0x1_0000_0000), VirtualAddress::from(0x77777000), proc.space.get_ttbr());

        unsafe {
            entry.to_kernel_addr().as_mut()
        }
    }

    // TODO this is temporary to suppress unused warnings
    #[allow(dead_code)]
    fn destroy_process(&mut self, proc: &mut Process) {
        proc.space.unmap_range(VirtualAddress::from(0), usize::MAX);
    }

    fn schedule(&mut self) {
        self.current += 1;
        if self.current >= self.processes.len() {
            self.current = 0;
        }

        let proc = &mut self.processes[self.current];
        unsafe {
            CURRENT_CONTEXT = &mut proc.context as *mut Context;
        }
    }

    fn page_fault(&mut self, far: u64) {
        self.processes[self.current].space.load_page(VirtualAddress::from(far));
    }
}

fn next_pid() -> Pid {
    let mut mutex = NEXT_PID.lock();
    let pid = *mutex;
    *mutex += 1;
    pid
}

const TEST_PROC1: &[u32] = &[0xd40000e1, 0xd503205f, 0x17ffffff];
const TEST_PROC2: &[u32] = &[0xd10043ff, 0xf90003e0, 0xf90007e1, 0x14000001, 0xd4000021, 0x17ffffff];

pub unsafe fn load_code(code: *mut u32, instructions: &[u32]) {
    for i in 0..instructions.len() {
        *code.add(i) = instructions[i];
    }
}

pub fn create_test_process() {
    unsafe {
        let ptr = PROCESS_MANAGER.lock().create_process();
        load_code(ptr.cast(), TEST_PROC1);

        let ptr = PROCESS_MANAGER.lock().create_process();
        load_code(ptr.cast(), TEST_PROC2);
    }
}

pub(crate) fn schedule() {
    PROCESS_MANAGER.lock().schedule();
}

pub(crate) fn page_fault_handler(far: u64) {
    PROCESS_MANAGER.lock().page_fault(far);
}

