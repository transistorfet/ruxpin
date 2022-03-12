
use crate::printkln;

use super::mmu::TranslationTable;

#[repr(C)]
pub struct Context {
    registers: [u64; 32],
    elr: u64,
    spsr: u64,
    ttbr: u64,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            registers: [0; 32],
            elr: 0,
            spsr: 0,
            ttbr: 0,
        }
    }
}

extern {
    pub fn create_context(context: &mut Context, sp: *mut u8, entry: *mut u8);
    pub fn start_multitasking();
}

#[no_mangle]
extern "C" fn handle_exception(sp: i64, esr: i64, elr: i64, far: i64) {
    printkln!("Handle an exception of {:x} for sp {:x}", esr, sp);

    if esr == 0x56000001 {
        printkln!("A SYSCALL!");
    } else {
        crate::fatal_error(esr, elr);
    }
}
 
impl Context {
    pub fn init(&mut self, sp: *mut u8, entry: *mut u8, ttbr: u64) {
        self.ttbr = ttbr;
        unsafe {
            create_context(self, sp, entry);
        }
    }
}

