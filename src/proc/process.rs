
use core::ptr;

use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::printkln;
use crate::mm::kmalloc::{kmalloc};
use crate::mm::vmalloc::VirtualAddressSpace;

use crate::arch::sync::Mutex;
use crate::arch::{Context, start_multitasking};


pub type Pid = i32;

pub struct Process {
    pid: Pid,
    context: Context,
    space: VirtualAddressSpace,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

pub static PROCESS_LIST: Mutex<Vec<Process>> = Mutex::new(Vec::new());
//pub static mut PROCESS_LIST: &mut [Option<Process>] = &mut [];

// TODO need to move this
#[no_mangle]
pub static mut CURRENT_CONTEXT: *mut Context = ptr::null_mut();


pub fn init_processes() {

}

pub fn create_process() -> *mut u8 {
    // TODO this is wrong temporarily
    let pid = 1;

    let mut processes = PROCESS_LIST.lock();

    processes.push(Process {
        pid,
        context: Default::default(),
        space: VirtualAddressSpace::new_user_space(),
    });

    let index = processes.len() - 1;
    let proc = &mut processes[index];
    let entry = proc.space.alloc_mapped(0x77777000, 4096);
    Context::init(&mut proc.context, (0x77777000 + 4096) as *mut u8, 0x77777000 as *mut u8, proc.space.get_ttbr());
    unsafe {
        // TODO this is temporary to bootstrap the context switching
        CURRENT_CONTEXT = &mut proc.context as *mut Context;
    }

    entry
}

pub fn create_test_process() {
    unsafe {
        //let size = 4096;
        //let ptr = kmalloc(size);
        //printkln!("Alloc: {:x}", ptr as usize);

        let ptr = create_process();

        let code: *mut u32 = ptr.cast();
        (*code) = 0xd4000021;
        (*code.offset(1)) = 0xd503205f;
        (*code.offset(2)) = 0x17ffffff;
        //let sp = ptr.offset(size as isize);


        //printkln!("SP: {:#x}", sp as u64);
        printkln!("Entry: {:#x}", ptr as u64);
        crate::printk::printk_dump(CURRENT_CONTEXT.cast(), 288);

        printkln!("Starting process");

        //asm!(
        //    "msr    SP_EL0, {new_sp}",
        //    new_sp = in(reg) new_sp,
        //);
        start_multitasking();
    }
}

