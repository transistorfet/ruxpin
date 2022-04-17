
use crate::misc::ceiling_div;

pub struct Bitmap<'a> {
    total_bits: usize,
    free_bits: usize,
    last_index: usize,
    table: &'a mut [u8],
}

impl<'a> Bitmap<'a> {
    pub fn new(total_bits: usize, table: &'a mut [u8]) -> Self {
        for byte in table.iter_mut() {
            *byte = 0;
        }

        Self {
            total_bits,
            free_bits: total_bits,
            last_index: 0,
            table,
        }
    }

    pub fn alloc(&mut self) -> Option<usize> {
        let mut i = self.last_index;

        loop {
            if i >= ceiling_div(self.total_bits, 8) {
                i = 0;
            }

            if self.table[i] != 0xff {
                let mut bit = 0;
                while bit < 7 && (self.table[i] & (0x01 << bit)) != 0 {
                    bit += 1;
                }
                self.table[i] |= 0x01 << bit;
                self.last_index = i;
                self.free_bits -= 1;
                return Some((i * 8) + bit);
            }

            i += 1;
            if i == self.last_index {
                return None;
            }
        }
    }

    pub fn free(&mut self, bitnum: usize) {
        let i = bitnum >> 3;
        let bit = bitnum & 0x7;
        self.table[i] &= !(0x01 << bit);
        self.free_bits += 1;
        // NOTE we could set last_index here, but not doing that might mean more contiguous chunks get allocated
        //if i < self.last_index {
        //    self.last_index = i;
        //}
    }

    pub const fn total_bits(&self) -> usize {
        self.total_bits
    }

    pub const fn free_bits(&self) -> usize {
        self.free_bits
    }
}


