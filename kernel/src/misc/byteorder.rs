
// On Aarch64, these functions should be converted into a "rev" instruction

#[inline(always)]
pub fn reverse_u16(source: u16) -> u16 {
      source >>  8
    | source <<  8
}

#[inline(always)]
pub fn reverse_u32(source: u32) -> u32 {
      (source >> 24) & 0x000000FF
    | (source >>  8) & 0x0000FF00
    | (source <<  8) & 0x00FF0000
    | (source << 24) & 0xFF000000
}

#[inline(always)]
pub fn reverse_u64(source: u64) -> u64 {
      (reverse_u32(source as u32) as u64) << 32
    | reverse_u32((source >> 32) as u32) as u64
}


#[cfg(target_endian = "little")]
mod arch {
    use super::{reverse_u16, reverse_u32, reverse_u64};

    #[inline(always)]
    pub fn convert_little_u16(source: u16) -> u16 { source }
    #[inline(always)]
    pub fn convert_little_u32(source: u32) -> u32 { source }
    #[inline(always)]
    pub fn convert_little_u64(source: u64) -> u64 { source }

    #[inline(always)]
    pub fn convert_big_u16(source: u16) -> u16 { reverse_u16(source) }
    #[inline(always)]
    pub fn convert_big_u32(source: u32) -> u32 { reverse_u32(source) }
    #[inline(always)]
    pub fn convert_big_u64(source: u64) -> u64 { reverse_u64(source) }
}

#[cfg(target_endian = "big")]
mod arch {
    use super::{reverse_u16, reverse_u32, reverse_u64};

    #[inline(always)]
    pub fn convert_little_u16(source: u16) -> u16 { reverse_u16(source) }
    #[inline(always)]
    pub fn convert_little_u32(source: u32) -> u32 { reverse_u32(source) }
    #[inline(always)]
    pub fn convert_little_u64(source: u64) -> u64 { reverse_u64(source) }

    #[inline(always)]
    pub fn convert_big_u16(source: u16) -> u16 { source }
    #[inline(always)]
    pub fn convert_big_u32(source: u32) -> u32 { source }
    #[inline(always)]
    pub fn convert_big_u64(source: u64) -> u64 { source }
}



#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct leu16(u16);

impl From<leu16> for u16 {
    fn from(source: leu16) -> u16 {
        arch::convert_little_u16(source.0)
    }
}

impl From<u16> for leu16 {
    fn from(source: u16) -> leu16 {
        leu16(arch::convert_little_u16(source))
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct leu32(u32);

impl From<leu32> for u32 {
    fn from(source: leu32) -> u32 {
        arch::convert_little_u32(source.0)
    }
}

impl From<u32> for leu32 {
    fn from(source: u32) -> leu32 {
        leu32(arch::convert_little_u32(source))
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct leu64(u64);

impl From<leu64> for u64 {
    fn from(source: leu64) -> u64 {
        arch::convert_little_u64(source.0)
    }
}

impl From<u64> for leu64 {
    fn from(source: u64) -> leu64 {
        leu64(arch::convert_little_u64(source))
    }
}


#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct beu16(u16);

impl From<beu16> for u16 {
    fn from(source: beu16) -> u16 {
        arch::convert_big_u16(source.0)
    }
}

impl From<u16> for beu16 {
    fn from(source: u16) -> beu16 {
        beu16(arch::convert_big_u16(source))
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct beu32(u32);

impl From<beu32> for u32 {
    fn from(source: beu32) -> u32 {
        arch::convert_big_u32(source.0)
    }
}

impl From<u32> for beu32 {
    fn from(source: u32) -> beu32 {
        beu32(arch::convert_big_u32(source))
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct beu64(u64);

impl From<beu64> for u64 {
    fn from(source: beu64) -> u64 {
        arch::convert_big_u64(source.0)
    }
}

impl From<u64> for beu64 {
    fn from(source: u64) -> beu64 {
        beu64(arch::convert_big_u64(source))
    }
}

