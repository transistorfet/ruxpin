
use core::arch::asm;

use crate::printkln;

#[repr(C)]
pub struct Context {
    x_registers: [u64; 32],
    v_registers: [u64; 64],     // TODO this should be u128, but they don't have a stable ABI, so I'm avoiding them for safety
    elr: u64,
    spsr: u64,
    ttbr: u64,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            x_registers: [0; 32],
            v_registers: [0; 64],
            elr: 0,
            spsr: 0,
            ttbr: 0,
        }
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

pub type IrqFlags = u64;

pub unsafe fn enable_irq(flags: IrqFlags) {
    asm!(
        "msr    DAIF, {:x}",
        in(reg) flags
    );
}

pub unsafe fn disable_irq() -> IrqFlags {
    let mut flags;
    asm!(
        "mrs    {:x}, DAIF",
        "msr    DAIFset, #0xf",
        out(reg) flags,
    );
    flags
}


extern {
    fn create_context(context: &mut Context, sp: *mut u8, entry: *mut u8);
    pub fn start_multitasking();
}

#[no_mangle]
extern "C" fn handle_exception(sp: i64, esr: i64, elr: i64, far: i64) {
    printkln!("Handle an exception of {:x} for sp {:x}", esr, sp);

    match esr >> 26 {
        // SVC from Aarch64
        0b010101 => {
            printkln!("A SYSCALL!");
            crate::proc::process::schedule();
        },

        // Instruction Abort from lower EL
        0b100000 => {
            if esr & 0b111100 == 0b001000 {
                printkln!("Instruction Abort caused by Access Flag (ie. load the data) at {:x}", far);
            }
            crate::fatal_error(esr, elr);
        },

        // Data Abort from lower EL
        0b100100 => {
            if esr & 0b111100 == 0b001000 {
                printkln!("Data Abort caused by Access Flag (ie. load the data) at {:x}", far);
            }
            crate::fatal_error(esr, elr);
        },
        _ => {
            crate::fatal_error(esr, elr);
        }
    }
}
 

