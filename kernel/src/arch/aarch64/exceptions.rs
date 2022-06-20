
use core::arch::asm;

use crate::api;
use crate::irqs;
use crate::tasklets;
use crate::{error, debug, trace};
use crate::printk::printk_dump;
use crate::proc::scheduler;

use super::types::VirtualAddress;
use super::context::{self, Context};


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
pub fn enable_all_irq() {
    unsafe {
        asm!("msr    DAIFclr, #0xf");
    }
}

#[allow(dead_code)]
pub fn disable_all_irq() {
    unsafe {
        asm!("msr    DAIFset, #0xf");
    }
}


#[no_mangle]
pub extern "C" fn fatal_error(context: &Context, elr: u64, esr: u64, far: u64) -> ! {
    let sp = context.get_stack();
    let table = context.get_translation_table();
    let far_addr = table.translate_addr(VirtualAddress::from(far));
    let elr_addr = table.translate_addr(VirtualAddress::from(elr));
    let pid = crate::proc::scheduler::get_current().lock().process_id;

    error!("\nFatal Error in PID {}: ESR: {:#010x}, FAR: {:#x}, ELR: {:#x}\n", pid, esr, far, elr);
    if let Ok(addr) = elr_addr {
        unsafe {
            let ptr: *const u32 = addr.to_kernel_addr().as_ptr();
            error!("Instruction: {:#010x}", *ptr);
        }
    }
    if let Ok(addr) = far_addr {
        unsafe {
            let ptr: *const u32 = addr.to_kernel_addr().as_ptr();
            error!("Fault Address: {:#010x}", *ptr);
        }
    }
    error!("\n{}", context);
    if u64::from(sp) != 0 {
        error!("\nStacktrace:");
        unsafe { printk_dump(u64::from(sp), 128); }
    }

    context::loop_forever();
}

#[no_mangle]
pub extern "C" fn fatal_kernel_error(_sp: u64, elr: u64, esr: u64, far: u64) -> ! {
    error!("\nFatal Kernel Error: ESR: {:#010x}, FAR: {:#x}, ELR: {:#x}", esr, far, elr);
    context::loop_forever();
}

#[no_mangle]
extern "C" fn handle_user_exception(context: &Context, elr: u64, esr: u64, far: u64, _sp: u64) {
    trace!("Handle an exception of ESR: {:x} from ELR: {:x}", esr, elr);

    match esr >> 26 {
        // SVC from Aarch64
        0b010101 => {
            api::handle_syscall();
        },

        // Instruction or Data Abort from lower EL
        0b100000 | 0b100100 => {
            if esr & 0b111100 == 0b001000 {
                trace!("Instruction or Data Abort caused by Access Flag at address {:x} (allocating new page)", far);
                page_fault_handler(far);
            } else if esr & 0b111100 == 0b001100 {
                trace!("Instruction or Data Abort caused by Permissions Flag at address {:x} (either copy-on-write or fault)", far);
                page_access_handler(far);
            } else {
                fatal_error(context, elr, esr, far);
            }
        },

        _ => {
            fatal_error(context, elr, esr, far);
        }
    }

    run_tasklets_with_interrupts();
    scheduler::check_restart_syscall();
}

#[no_mangle]
extern "C" fn handle_user_irq(_context: &Context, _elr: u64, _esr: u64, _far: u64, _sp: u64) {
    //trace!("Handle an irq of {:x} for sp {:x}", _esr, _sp);

    irqs::handle_irqs();

    run_tasklets_with_interrupts();
    scheduler::check_restart_syscall();
}

#[no_mangle]
extern "C" fn handle_kernel_exception(sp: u64, elr: u64, esr: u64, far: u64) {
    debug!("Handle a kernel exception of {:x} for far {:x} at {:x}", esr, far, elr);

    match esr >> 26 {
        // Instruction or Data Abort from lower EL
        0b100000 | 0b100100 | 0b100101 => {
            if esr & 0b111100 == 0b001000 {
                trace!("Instruction or Data Abort caused by Access Flag at address {:x} (allocating new page)", far);
                page_fault_handler(far);
            } else {
                fatal_kernel_error(sp, elr, esr, far);
            }
        },

        _ => {
            fatal_kernel_error(sp, elr, esr, far);
        }
    }
}

#[no_mangle]
extern "C" fn handle_kernel_irq(_context: &Context, _elr: u64, _esr: u64, _far: u64, _sp: u64) {
    //trace!("Handle an irq of {:x} for sp {:x}", _esr, _sp);

    irqs::handle_irqs();

    run_tasklets_with_interrupts();
}

fn run_tasklets_with_interrupts() {
    enable_all_irq();
    tasklets::run();
    disable_all_irq();
}

fn page_fault_handler(far: u64) {
    let current = scheduler::get_current();
    current.try_lock().unwrap().space.try_lock().unwrap().alloc_page_at(VirtualAddress::from(far)).unwrap_or_else(|_| scheduler::abort(scheduler::get_current()));
}

fn page_access_handler(far: u64) {
    let current = scheduler::get_current();
    current.try_lock().unwrap().space.try_lock().unwrap().copy_on_write_at(VirtualAddress::from(far)).unwrap_or_else(|_| scheduler::abort(scheduler::get_current()));
}

