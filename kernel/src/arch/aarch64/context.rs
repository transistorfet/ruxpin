
use super::types::VirtualAddress;

extern "C" {
    // These definitions are in aarch64/exceptions.s
    fn _create_context(context: &mut Context, sp: u64, entry: u64);
    pub fn _start_multitasking() -> !;
}

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
    pub fn init(&mut self, sp: VirtualAddress, entry: VirtualAddress, ttbr: u64) {
        self.ttbr = ttbr;
        unsafe {
            _create_context(self, u64::from(sp), u64::from(entry));
        }
    }
}

pub fn start_multitasking() -> ! {
    unsafe {
        _start_multitasking();
    }
}

