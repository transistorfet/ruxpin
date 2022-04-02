
use core::str;

use crate::errors::KernelError;

pub struct StrArray<const LENGTH: usize> {
    len: usize,
    array: [u8; LENGTH],
}

impl<const LENGTH: usize> StrArray<LENGTH> {
    pub fn new() -> Self {
        Self {
            len: 0,
            array: [0; LENGTH],
        }
    }

    pub fn copy_into(&mut self, s: &str) {
        self.len = 0;
        for ch in s.as_bytes() {
            if self.len >= LENGTH {
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

impl<const LENGTH: usize> TryInto<StrArray<LENGTH>> for &str {
    type Error = KernelError;

    fn try_into(self) -> Result<StrArray<LENGTH>, KernelError> {
        let mut array = StrArray::new();
        array.copy_into(self);
        Ok(array)
    }
}

