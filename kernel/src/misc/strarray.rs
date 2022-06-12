
use core::str;
use core::mem;

use crate::misc::memory;
use crate::misc::align_up;
use crate::arch::types::VirtualAddress;
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

    pub fn as_mut(&mut self) -> &mut [u8] {
        &mut self.array
    }

    pub unsafe fn set_len(&mut self, length: usize) {
        self.len = length;
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


pub struct ArrayOfStrings<const LENGTH: usize, const WORDS: usize> {
    buffer_len: usize,
    buffer: [u8; LENGTH],
    offsets_len: usize,
    offsets: [usize; WORDS],
}

impl<const LENGTH: usize, const WORDS: usize> ArrayOfStrings<LENGTH, WORDS> {
    pub fn new() -> Self {
        Self {
            buffer_len: 0,
            buffer: [0; LENGTH],
            offsets_len: 0,
            offsets: [0; WORDS],
        }
    }

    pub fn new_parsed(args: &[&str]) -> Self {
        let mut strings = Self::new();
        strings.copy_into(args);
        strings
    }

    pub fn offset_len(&self) -> usize {
        self.offsets_len
    }

    pub fn calculate_size(&self) -> usize {
        align_up(self.buffer_len + ((self.offsets_len + 1) * mem::size_of::<*const u8>()), mem::size_of::<usize>())
    }

    pub fn copy_into(&mut self, args: &[&str]) {
        let mut i = 0;
        let mut j = 0;
        for arg in args.iter() {
            self.offsets[j] = i;
            j += 1;

            (&mut self.buffer[i..(i + arg.len())]).copy_from_slice(arg.as_bytes());
            i += arg.len();
            self.buffer[i] = 0;
            i += 1;
        }

        self.buffer_len = i;
        self.offsets_len = j;
    }

    pub fn marshall(&self, dest: &mut [u8], base_addr: VirtualAddress) {
        let buffer_start = (self.offsets_len + 1) * mem::size_of::<usize>();
        let dest_usize: &mut [usize] = unsafe { memory::cast_to_slice_mut(dest) };

        for i in 0..self.offsets_len {
            dest_usize[i] = usize::from(base_addr.add(buffer_start).add(self.offsets[i]));
        }
        dest_usize[self.offsets_len] = 0;

        for i in 0..self.buffer_len {
            dest[buffer_start + i] = self.buffer[i];
        }
    }
}

pub type StandardArrayOfStrings = ArrayOfStrings<2048, 20>;

