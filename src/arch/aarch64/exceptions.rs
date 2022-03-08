
use crate::printkln;

#[no_mangle]
pub extern "C" fn handle_exception(sp: i64, esr: i64, _far: i64) {
    printkln!("Handle an exception of {:x} for sp {:x}", esr, sp);

    if esr == 0x56000001 {
        printkln!("A SYSCALL!");
    }
}
 
