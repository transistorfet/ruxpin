
use core::str;

pub struct StrArray<const length: usize> {
    len: usize,
    array: [u8; length],
}

impl<const length: usize> StrArray<length> {
    pub fn new() -> Self {
        Self {
            len: 0,
            array: [0; length],
        }
    }

    pub fn copy_into(&mut self, s: &str) {
        self.len = 0;
        for ch in s.as_bytes() {
            if self.len >= length {
                return;
            }
            self.array[self.len] = *ch;
            self.len += 1;
        }
    }

    pub fn as_str<'a>(&'a self) -> &'a str {
        unsafe {
            str::from_utf8_unchecked(&self.array[..self.len])
        }
    }
}


