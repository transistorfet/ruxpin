
use crate::printkln;

#[no_mangle]
pub extern "C" fn handle_exception(sp: i64, esr: i64, elr: i64, far: i64) {
    printkln!("Handle an exception of {:x} for sp {:x}", esr, sp);

    if esr == 0x56000001 {
        printkln!("A SYSCALL!");
    } else {
        crate::fatal_error(esr, elr);
    }
}
 
