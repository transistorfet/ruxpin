
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

pub unsafe fn enable_all_irq() {
    asm!("msr    DAIFclr, #0xf");
}

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

#[no_mangle]
extern "C" fn handle_irq(sp: i64, esr: i64, elr: i64, far: i64) {
    printkln!("Handle an irq of {:x} for sp {:x}", esr, sp);

    unsafe {
        IRQ_HANDLER();
    }

    crate::proc::process::schedule();
}

