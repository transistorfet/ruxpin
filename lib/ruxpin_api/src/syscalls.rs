
#[repr(usize)]
#[derive(Copy, Clone, Debug)]
pub enum SyscallFunction {
    None,
    Open,
    Close,
    Read,
    Write,
}

#[derive(Clone, Debug)]
pub struct SyscallRequest {
    pub function: SyscallFunction,
    pub args: [usize; 6],
    pub result: usize,
    pub error: bool,
}

impl Default for SyscallRequest {
    fn default() -> Self {
        Self {
            function: SyscallFunction::None,
            args: [0; 6],
            result: 0,
            error: false,
        }
    }
}

#[macro_export]
macro_rules! syscall_encode {
    ($syscall:ident, $i:ident, $name:ident: FileDesc) => {
        $i += 1;
        $syscall.args[$i - 1] = $name.0;
    };

    ($syscall:ident, $i:ident, $name:ident: OpenFlags) => {
        $i += 1;
        $syscall.args[$i - 1] = $name.0 as usize;
    };

    ($syscall:ident, $i:ident, $name:ident: FileAccess) => {
        $i += 1;
        $syscall.args[$i - 1] = $name.0 as usize;
    };

    ($syscall:ident, $i:ident, $name:ident: &[$type:ty]) => {
        $i += 2;
        $syscall.args[$i - 2] = $name.as_ptr() as usize;
        $syscall.args[$i - 1] = $name.len();
    };

    ($syscall:ident, $i:ident, $name:ident: &mut [$type:ty]) => {
        $i += 2;
        $syscall.args[$i - 2] = $name.as_ptr() as usize;
        $syscall.args[$i - 1] = $name.len();
    };

    ($syscall:ident, $i:ident, $name:ident: &str) => {
        $i += 2;
        $syscall.args[$i - 2] = $name.as_bytes().as_ptr() as usize;
        $syscall.args[$i - 1] = $name.as_bytes().len();
    };
}

#[macro_export]
macro_rules! syscall_decode {
    ($syscall:ident, $i:ident, $name:ident: FileDesc) => {
        $i += 1;
        let $name = FileDesc($syscall.args[$i - 1]);
    };

    ($syscall:ident, $i:ident, $name:ident: OpenFlags) => {
        $i += 1;
        let $name = OpenFlags($syscall.args[$i - 1] as u16);
    };

    ($syscall:ident, $i:ident, $name:ident: FileAccess) => {
        $i += 1;
        let $name = FileAccess($syscall.args[$i - 1] as u16);
    };

    ($syscall:ident, $i:ident, $name:ident: &[$type:ty]) => {
        $i += 2;
        let $name = unsafe {
            core::slice::from_raw_parts($syscall.args[$i - 2] as *const $type, $syscall.args[$i - 1])
        };
    };

    ($syscall:ident, $i:ident, $name:ident: &mut [$type:ty]) => {
        $i += 2;
        let $name = unsafe {
            core::slice::from_raw_parts_mut($syscall.args[$i - 2] as *mut $type, $syscall.args[$i - 1])
        };
    };

    ($syscall:ident, $i:ident, $name:ident: &str) => {
        $i += 2;
        let $name = unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts($syscall.args[$i - 2] as *const u8, $syscall.args[$i - 1]))
        };
    };
}


