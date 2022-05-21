
use core::arch::asm;

use crate::irqs;
use crate::printkln;
use crate::printk::printk_dump;

use super::context::Context;
use super::types::{VirtualAddress, PhysicalAddress};


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

    printkln!("\nFatal Error in PID {}: ESR: {:#010x}, FAR: {:#x}, ELR: {:#x}\n", pid, esr, far, elr);
    if let Ok(addr) = elr_addr {
        unsafe {
            let ptr: *const u32 = addr.to_kernel_addr().as_ptr();
            printkln!("Instruction: {:#010x}", *ptr);
        }
    }
    if let Ok(addr) = far_addr {
        unsafe {
            let ptr: *const u32 = addr.to_kernel_addr().as_ptr();
            printkln!("Fault Address: {:#010x}", *ptr);
        }
    }
    printkln!("\n{}", context);
    printkln!("\nStacktrace:");
    unsafe { printk_dump(u64::from(context.get_stack()), 128); }
    loop {}
}

#[no_mangle]
pub extern "C" fn fatal_kernel_error(sp: u64, elr: u64, esr: u64, far: u64) -> ! {
    printkln!("\nFatal Error: ESR: {:#010x}, FAR: {:#x}, ELR: {:#x}", esr, far, elr);
    printkln!("\nStacktrace:");
    unsafe { printk_dump(sp, 128); }
    loop {}
}

#[no_mangle]
extern "C" fn handle_user_exception(context: &Context, elr: u64, esr: u64, far: u64, _sp: u64) {
    //printkln!("Handle an exception of {:x} for sp {:x}", esr, sp);

    match esr >> 26 {
        // SVC from Aarch64
        0b010101 => {
            use crate::api::handle_syscall;
            handle_syscall();
        },

        // Instruction or Data Abort from lower EL
        0b100000 | 0b100100 => {
            if esr & 0b111100 == 0b001000 {
                printkln!("Instruction or Data Abort caused by Access Flag at address {:x} (allocating new page)", far);
                page_fault_handler(far);
            } else if esr & 0b111100 == 0b001100 {
                printkln!("Instruction or Data Abort caused by Permissions Flag at address {:x} (either copy-on-write or fault)", far);
                page_access_handler(far);
            } else {
                fatal_error(context, elr, esr, far);
            }
        },

        _ => {
            fatal_error(context, elr, esr, far);
        }
    }

    enable_all_irq();
    crate::tasklets::run_tasklets();
    disable_all_irq();
}

#[no_mangle]
extern "C" fn handle_kernel_exception(sp: u64, elr: u64, esr: u64, far: u64) {
    printkln!("Handle a kernel exception of {:x} for far {:x} at {:x}", esr, far, elr);

    match esr >> 26 {
        // Instruction or Data Abort from lower EL
        0b100000 | 0b100100 | 0b100101 => {
            if esr & 0b111100 == 0b001000 {
                printkln!("Instruction or Data Abort caused by Access Flag at address {:x} (allocating new page)", far);
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
extern "C" fn handle_irq(context: &Context, _elr: u64, _esr: u64, _far: u64, _sp: u64) {
    //printkln!("Handle an irq of {:x} for sp {:x}", _esr, _sp);

    irqs::handle_irqs();

    enable_all_irq();
    crate::tasklets::run_tasklets();
    disable_all_irq();
}

fn page_fault_handler(far: u64) {
    let current = crate::proc::scheduler::get_current();
    current.try_lock().unwrap().space.try_lock().unwrap().alloc_page_at(VirtualAddress::from(far)).unwrap();
}

fn page_access_handler(far: u64) {
    //let current = crate::proc::scheduler::get_current();
    //current.try_lock().unwrap().space.try_lock().unwrap().copy_on_write_at(VirtualAddress::from(far)).unwrap();
}

