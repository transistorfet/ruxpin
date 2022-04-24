
use core::str;
use core::slice;


static mut _SYS_ARGV: &'static [*const u8] = &[];
static mut _SYS_ENVP: &'static [*const u8] = &[];


pub fn args() -> Args {
    Args {
        args: unsafe { _SYS_ARGV },
        position: 0,
    }
}

pub struct Args {
    args: &'static [*const u8],
    position: usize,
}

impl Args {
    pub(crate) fn set_args(argc: isize, argv: *const *const u8) {
        unsafe {
            _SYS_ARGV = slice::from_raw_parts(argv, argc as usize);
        }
    }

    pub fn get(&self, index: usize) -> &'static str {
        make_cstr(self.args[index])
    }
}

impl Iterator for Args {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.args.len() {
            let result = self.get(self.position);
            self.position += 1;
            Some(result)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for Args {
    fn len(&self) -> usize {
        self.args.len()
    }
}


pub fn vars() -> Vars {
    Vars {
        vars: unsafe { _SYS_ENVP },
        position: 0,
    }
}

pub struct Vars {
    vars: &'static [*const u8],
    position: usize,
}

impl Vars {
    pub(crate) fn set_vars(envp: *const *const u8) {
        let mut envc = 0;
        while unsafe { *(envp.add(envc)) }.is_null() {
            envc += 1;
        }

        unsafe {
            _SYS_ENVP = slice::from_raw_parts(envp, envc);
        }
    }
}

impl Iterator for Vars {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.vars.len() {
            let result = make_cstr(self.vars[self.position]);
            self.position += 1;
            Some(result)
        } else {
            None
        }
    }
}



fn cstrlen(cstr: *const u8) -> usize {
    let mut i = 0;
    while unsafe { *(cstr.add(i)) } != 0 {
        i += 1;
    }
    i
}

fn make_cstr(cstr: *const u8) -> &'static str {
    unsafe {
        str::from_utf8_unchecked(slice::from_raw_parts(cstr, cstrlen(cstr)))
    }
}

