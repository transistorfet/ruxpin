
use core::ptr;

use alloc::vec::Vec;

use crate::arch::{create_context, start_multitasking};
use crate::printkln;
use crate::mm::kmalloc::{kmalloc};


pub type Pid = i32;

pub struct Process {
    pid: Pid,
    stack: *mut u8,
    //page_table: PageTable,
}

impl Default for Process {
    fn default() -> Self {
        Process {
            pid: -1,
            stack: ptr::null_mut()
        }
    }
}

pub static mut process_list: &[Process] = &[];

#[no_mangle]
pub static mut PROCESS_SAVED_SP: *mut u8 = ptr::null_mut();

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
        let new_sp = create_context(sp, ptr);
        printkln!("SP: {:#x}", new_sp as u64);
        crate::printk::printk_dump(new_sp, 288);

        printkln!("Starting process");

        //asm!(
        //    "msr    SP_EL0, {new_sp}",
        //    new_sp = in(reg) new_sp,
        //);
        start_multitasking();
    }
}
 
#[no_mangle]
pub extern "C" fn handle_exception(sp: i64, esr: i64, _far: i64) {
    printkln!("Handle an exception of {:x} for sp {:x}", esr, sp);
}

