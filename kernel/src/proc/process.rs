
use alloc::vec::Vec;
use alloc::sync::Arc;

use ruxpin_api::types::UserID;

use crate::mm::MemoryPermissions;
use crate::mm::vmalloc::VirtualAddressSpace;

use crate::arch::Context;
use crate::sync::Spinlock;
use crate::arch::types::VirtualAddress;
use crate::fs::filedesc::FileDescriptors;
use crate::fs::types::Vnode;

pub type Pid = i32;

pub struct ProcessRecord {
    // TODO I don't like that these are all pub... I might need to either isolate this more or change how things interact with processes
    pub pid: Pid,
    pub current_uid: UserID,
    pub space: VirtualAddressSpace,
    pub cwd: Option<Vnode>,
    pub files: FileDescriptors,
    pub context: Context,
}

pub type Process = Arc<Spinlock<ProcessRecord>>;

struct ProcessManager {
    processes: Vec<Process>,
    //schedule: UnownedLinkedList<Process>,
    current: usize,
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
            current: 0,
        }
    }

    pub fn create_process(&mut self) -> Process {
        let pid = next_pid();

        self.processes.push(Arc::new(Spinlock::new(ProcessRecord {
            pid,
            current_uid: 0,
            space: VirtualAddressSpace::new_user_space(),
            cwd: None,
            files: FileDescriptors::new(),
            context: Default::default(),
        })));

        self.current = self.processes.len() - 1;

        self.processes[self.current].clone()
    }

    fn create_test_process(&mut self) -> Process {
        let proc = self.create_process();

        {
            let mut locked_proc = proc.lock();

            // Allocate text segment
            locked_proc.space.alloc_mapped(MemoryPermissions::ReadExecute, VirtualAddress::from(0x77777000), 4096);
            // Allocate stack segment
            //proc.space.alloc_mapped(MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);
            locked_proc.space.map_on_demand(MemoryPermissions::ReadWrite, VirtualAddress::from(0xFF000000), 4096 * 4096);
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

    pub fn get_current_proc(&self) -> Process {
        self.processes[self.current].clone()
    }

    fn schedule(&mut self) {
        self.current += 1;
        if self.current >= self.processes.len() {
            self.current = 0;
        }

        Context::switch_current_context(&mut self.processes[self.current].lock().context);
    }

    fn page_fault(&mut self, far: u64) {
        self.processes[self.current].lock().space.load_page(VirtualAddress::from(far));
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
    PROCESS_MANAGER.lock().create_process()
}

pub fn get_current_proc() -> Process {
    PROCESS_MANAGER.lock().get_current_proc()
}

pub(crate) fn schedule() {
    PROCESS_MANAGER.lock().schedule();
}

pub(crate) fn page_fault_handler(far: u64) {
    PROCESS_MANAGER.lock().page_fault(far);
}

