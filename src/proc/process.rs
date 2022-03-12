
use core::ptr;

use alloc::boxed::Box;

use crate::printkln;
use crate::mm::kmalloc::{kmalloc};
use crate::mm::vmalloc::VirtualAddressSpace;

use crate::arch::{Context, start_multitasking};


pub type Pid = i32;

pub struct Process {
    pid: Pid,
    context: Context,
    space: VirtualAddressSpace,
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

pub fn create_process() -> *mut u8 {
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
            space: VirtualAddressSpace::new_user_space(),
        });

        let proc = PROCESS_LIST[i].as_mut().unwrap();
        let entry = proc.space.alloc_page();
        proc.space.map_existing_page(0x77777000 as *mut u8, entry);
        Context::init(&mut proc.context, (0x77777000 + 4096) as *mut u8, 0x77777000 as *mut u8, proc.space.get_ttbr());
        // TODO this is temporary to bootstrap the context switching
        CURRENT_CONTEXT = &mut proc.context as *mut Context;

        entry
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

