
use crate::printkln;
use crate::mm::kmalloc::{kmalloc};

extern {
    fn _create_context(sp: *mut i8, entry: *mut i8) -> *mut i8;
    fn _restore_context();
    fn _start_multitasking();
}

//pub static mut PROCESS_SAVED_SP: *mut i8 = ptr::null_mut();

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
        let new_sp = _create_context(sp, ptr);
        printkln!("SP: {:#x}", new_sp as u64);
        crate::printk::printk_dump(new_sp, 288);

        printkln!("Starting process");

        //asm!(
        //    "msr    SP_EL0, {new_sp}",
        //    new_sp = in(reg) new_sp,
        //);
        _start_multitasking();
    }
}
 
#[no_mangle]
pub extern "C" fn handle_exception(sp: i64, esr: i64, _far: i64) {
    printkln!("Handle an exception of {:x} for sp {:x}", esr, sp);
}

