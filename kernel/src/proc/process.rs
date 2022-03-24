
use core::ptr;

use alloc::vec::Vec;

use crate::printkln;
use crate::mm::MemoryAccess;
use crate::mm::vmalloc::VirtualAddressSpace;

use crate::arch::sync::Mutex;
use crate::arch::types::VirtualAddress;
use crate::arch::{Context, start_multitasking};


pub type Pid = i32;

pub struct Process {
    pid: Pid,
    context: Context,
    space: VirtualAddressSpace,
}

struct ProcessManager {
    processes: Vec<Process>,
    current: usize,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

static PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());

// TODO need to move this
#[no_mangle]
pub static mut CURRENT_CONTEXT: *mut Context = ptr::null_mut();


pub fn init_processes() {

}

impl ProcessManager {
    const fn new() -> Self {
        Self {
            processes: Vec::new(),
            current: 0,
        }
    }

    fn create_process(&mut self) -> *mut u8 {
        // TODO this is wrong temporarily
        let pid = 1;

        self.processes.push(Process {
            pid,
            context: Default::default(),
            space: VirtualAddressSpace::new_user_space(),
        });

        self.current = self.processes.len() - 1;
        let proc = &mut self.processes[self.current];
        // Allocate text segment
        let entry = proc.space.alloc_mapped(MemoryAccess::ReadExecute, VirtualAddress::from(0x77777000), 4096);
        // Allocate stack segment
        proc.space.alloc_mapped(MemoryAccess::ReadWrite, VirtualAddress::from(0xFFFFF000), 4096);
        Context::init(&mut proc.context, VirtualAddress::from(0x1_0000_0000), VirtualAddress::from(0x77777000), proc.space.get_ttbr());

        unsafe {
            // TODO this is temporary to bootstrap the context switching
            CURRENT_CONTEXT = &mut proc.context as *mut Context;
        }

        entry
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
            // TODO this is temporary to bootstrap the context switching
            CURRENT_CONTEXT = &mut proc.context as *mut Context;
        }
    }
}

pub fn schedule() {
    PROCESS_MANAGER.lock().schedule();
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


        printkln!("Entry: {:#x}", ptr as u64);
        crate::printk::printk_dump(CURRENT_CONTEXT.cast(), 768 + 32);

        printkln!("Starting process");

        start_multitasking();
    }
}

