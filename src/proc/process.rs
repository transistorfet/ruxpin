
use core::ptr;

use alloc::boxed::Box;

use crate::printkln;
use crate::mm::kmalloc::{kmalloc};

use crate::arch::{Context, start_multitasking};


pub type Pid = i32;

pub struct Process {
    pid: Pid,
    context: Context,
}

impl Default for Process {
    fn default() -> Self {
        Process {
            pid: -1,
            context: Default::default(),
        }
    }
}

pub static mut PROCESS_LIST: &mut [Option<Process>] = &mut [];

// TODO need to move this
#[no_mangle]
pub static mut CURRENT_CONTEXT: *mut Context = ptr::null_mut();


pub fn init_processes() {
    let process_list: Box<[Option<Process>; 20]> = Default::default();

    unsafe {
        PROCESS_LIST = Box::leak(process_list);
    }
}

pub fn create_process(sp: *mut u8, entry: *mut u8) {
    let i = match find_empty_process() {
        Some(i) => i,
        None => panic!("No more processes left"),
    };

    // TODO this is wrong temporarily
    let pid = 1;

    unsafe {
        PROCESS_LIST[i] = Some(Process {
            pid,
            context: Default::default(),
        });

        let context = &mut PROCESS_LIST[i].as_mut().unwrap().context;
        Context::init(context, sp, entry);
        // TODO this is temporary to bootstrap the context switching
        CURRENT_CONTEXT = context as *mut Context;
    }
}

fn find_empty_process() -> Option<usize> {
    unsafe {
        for i in 0..PROCESS_LIST.len() {
            if PROCESS_LIST[i].is_none() {
                return Some(i);
            }
        }
    }

    None
}


pub fn create_test_process() {
    unsafe {
        let size = 4096;
        let ptr = kmalloc(size);
        printkln!("Alloc: {:x}", ptr as usize);

        let code: *mut u32 = ptr.cast();
        (*code) = 0xd4000021;
        (*code.offset(1)) = 0xd503205f;
        (*code.offset(2)) = 0x17ffffff;
        let sp = ptr.offset(size as isize);

        create_process(sp, ptr);

        printkln!("SP: {:#x}", sp as u64);
        crate::printk::printk_dump(CURRENT_CONTEXT.cast(), 288);

        printkln!("Starting process");

        //asm!(
        //    "msr    SP_EL0, {new_sp}",
        //    new_sp = in(reg) new_sp,
        //);
        start_multitasking();
    }
}

