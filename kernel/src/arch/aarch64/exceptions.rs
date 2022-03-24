
use core::arch::asm;

use crate::printkln;

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

#[allow(dead_code)]
pub unsafe fn enable_all_irq() {
    asm!("msr    DAIFclr, #0xf");
}

#[allow(dead_code)]
pub unsafe fn disable_all_irq() {
    asm!("msr    DAIFset, #0xf");
}

static mut IRQ_HANDLER: fn() = default_handler;

pub fn register_irq(func: fn()) {
    unsafe {
        IRQ_HANDLER = func;
    }
}

const fn default_handler() {
    /* Do Nothing */
}


#[no_mangle]
extern "C" fn handle_exception(_context: u64, elr: u64, esr: u64, far: u64, sp: u64) {
    printkln!("Handle an exception of {:x} for sp {:x}", esr, sp);

    match esr >> 26 {
        // SVC from Aarch64
        0b010101 => {
            printkln!("A SYSCALL!");
            crate::proc::process::schedule();
        },

        // Instruction or Data Abort from lower EL
        0b100000 | 0b100100 => {
            if esr & 0b111100 == 0b001000 {
                printkln!("Instruction Abort caused by Access Flag (ie. load the data) at {:x}", far);
                use crate::proc::process::page_fault_handler;
                page_fault_handler(far);
            } else {
                crate::fatal_error(elr, esr, far);
            }
        },

        _ => {
            crate::fatal_error(elr, esr, far);
        }
    }
}

#[no_mangle]
extern "C" fn handle_irq(_context: u64, _elr: u64, esr: u64, _far: u64, sp: u64) {
    printkln!("Handle an irq of {:x} for sp {:x}", esr, sp);

    unsafe {
        IRQ_HANDLER();
    }

    //crate::proc::process::schedule();
}

