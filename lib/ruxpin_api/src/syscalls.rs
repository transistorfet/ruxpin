
#[repr(usize)]
#[derive(Copy, Clone, Debug, PartialEq)]
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


/*
macro_rules! call_syscall {
    ($arg1:expr) => {

    }
}


macro_rules! define_syscall {
    ($name:ident ( $arg1:ident ), $( $body:tt* )* ) => {
        pub fn $name($arg1: u32) -> Result<(), u32> {

        }
    }
}

fn test() {
    call_syscall!(5);
}

define_syscall!(api_print_number(num),
    //nothing much
);

macro_rules! syscall_args {
    /*
    ($syscall:ident, $name0:ident: $type0:ty, $name1:ident: $type1:ty) => {
        let $name0 = <$type0>::from($syscall.args[0]);
        let $name1 = <$type1>::from($syscall.args[1]);
    }
    */

    ($syscall:ident, $name:ident: $type:ty, $( $remain:tt )*) => {
        //let $name = <$type>::from($syscall.args[0]);
        syscall_encode!($syscall, 0, $name: $type);
        syscall_args!($syscall, $( $remain )*);
    };

    ($syscall:ident, $name:ident: $type:ty) => {
        //let $name = <$type>::from($syscall.args[0]);
        syscall_encode!($syscall, 0, $name: $type);
    }

    //($syscall:ident, $($remain:tt)*) => {
    //    syscall_args!($syscall, $($remain)*)
    //};
}
*/

