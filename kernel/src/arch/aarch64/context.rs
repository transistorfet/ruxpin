
use core::ptr;
use core::fmt;
use core::arch::asm;

use ruxpin_api::syscalls::{SyscallRequest, SyscallFunction};

use super::types::VirtualAddress;

extern "C" {
    // These definitions are in aarch64/exceptions.s
    fn _create_context(context: &mut Context, sp: u64, entry: u64);
    pub fn _start_multitasking() -> !;
}

#[no_mangle]
pub static mut CURRENT_CONTEXT: *mut Context = ptr::null_mut();

#[repr(C)]
#[derive(Clone)]
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

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, reg) in self.x_registers.iter().enumerate() {
            write!(f, "x{:02}: {:#018x} ", i, reg)?;
            if i % 4 == 3 {
                write!(f, "\n")?;
            }
        }

        write!(f, "ELR: {:#018x} ", self.elr)?;
        write!(f, "SPSR: {:#018x} ", self.spsr)?;
        write!(f, "TTBR: {:#018x} ", self.ttbr)?;
        write!(f, "\n")?;
        Ok(())
    }
}

impl Context {
    pub fn init(&mut self, entry: VirtualAddress, sp: VirtualAddress, ttbr: u64) {
        self.set_ttbr(ttbr);
        unsafe {
            _create_context(self, u64::from(sp), u64::from(entry));
        }
    }

    pub fn set_ttbr(&mut self, ttbr: u64) {
        self.ttbr = ttbr;
    }

    pub fn get_stack(&self) -> VirtualAddress {
        VirtualAddress::from(self.x_registers[31])
    }

    pub fn write_syscall_result(&mut self, syscall: &SyscallRequest) {
        self.x_registers[0] = syscall.result as u64;
        self.x_registers[1] = syscall.error as u64;
    }

    pub fn write_result(&mut self, result: Result<usize, usize>) {
        match result {
            Ok(num) => {
                self.x_registers[0] = num as u64;
                self.x_registers[1] = false as u64;
            },
            Err(num) => {
                self.x_registers[0] = num as u64;
                self.x_registers[1] = true as u64;
            },
        }
    }

    pub fn write_args(&mut self, argc: usize, argv: VirtualAddress, envp: VirtualAddress) {
        self.x_registers[0] = argc as u64;
        self.x_registers[1] = argv.into();
        self.x_registers[2] = envp.into();
    }
}

impl Context {
    pub fn dump_current() {
        unsafe {
            crate::printkln!("{}", &*CURRENT_CONTEXT);
        }
    }

    pub fn switch_current_context(new_context: &mut Context) {
        unsafe {
            // Update TTBR0 before the context switch, so we can restart a syscall in progress
            asm!(
                "msr     TTBR0_EL1, {ttbr}",
                ttbr = in(reg) new_context.ttbr,
            );

            CURRENT_CONTEXT = new_context as *mut Context;
        }
    }

    pub fn syscall_from_current_context() -> SyscallRequest {
        unsafe {
            (&*CURRENT_CONTEXT).into()
        }
    }

    pub fn write_syscall_result_to_current_context(syscall: &SyscallRequest) {
        unsafe {
            (&mut *CURRENT_CONTEXT).write_syscall_result(syscall);
        }
    }
}


impl From<&Context> for SyscallRequest {
    fn from(context: &Context) -> SyscallRequest {
        SyscallRequest {
            function: unsafe { *(&context.x_registers[6] as *const u64 as *const SyscallFunction) },
            args: [
                context.x_registers[0] as usize,
                context.x_registers[1] as usize,
                context.x_registers[2] as usize,
                context.x_registers[3] as usize,
                context.x_registers[4] as usize,
                context.x_registers[5] as usize,
            ],
            result: 0,
            error: false,
        }
    }
}


pub fn start_multitasking() -> ! {
    unsafe {
        _start_multitasking();
    }
}

