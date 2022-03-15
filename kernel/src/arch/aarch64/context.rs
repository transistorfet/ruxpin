
extern {
    // These definitions are in aarch64/exceptions.s
    fn create_context(context: &mut Context, sp: *mut u8, entry: *mut u8);
    pub fn start_multitasking();
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
    pub fn init(&mut self, sp: *mut u8, entry: *mut u8, ttbr: u64) {
        self.ttbr = ttbr;
        unsafe {
            create_context(self, sp, entry);
        }
    }
}

