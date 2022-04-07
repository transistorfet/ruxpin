
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
    pub fn from_little_u16(source: u16) -> u16 { source }
    #[inline(always)]
    pub fn from_little_u32(source: u32) -> u32 { source }
    #[inline(always)]
    pub fn from_little_u64(source: u64) -> u64 { source }

    #[inline(always)]
    pub fn from_big_u16(source: u16) -> u16 { reverse_u16(source) }
    #[inline(always)]
    pub fn from_big_u32(source: u32) -> u32 { reverse_u32(source) }
    #[inline(always)]
    pub fn from_big_u64(source: u64) -> u64 { reverse_u64(source) }
}

#[cfg(target_endian = "big")]
mod arch {
    use super::{reverse_u16, reverse_u32, reverse_u64};

    #[inline(always)]
    pub fn from_little_u16(source: u16) -> u16 { reverse_u16(source) }
    #[inline(always)]
    pub fn from_little_u32(source: u32) -> u32 { reverse_u32(source) }
    #[inline(always)]
    pub fn from_little_u64(source: u64) -> u64 { reverse_u64(source) }

    #[inline(always)]
    pub fn from_big_u16(source: u16) -> u16 { source }
    #[inline(always)]
    pub fn from_big_u32(source: u32) -> u32 { source }
    #[inline(always)]
    pub fn from_big_u64(source: u64) -> u64 { source }
}



#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct leu16(u16);

impl From<leu16> for u16 {
    fn from(source: leu16) -> u16 {
        arch::from_little_u16(source.0)
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct leu32(u32);

impl From<leu32> for u32 {
    fn from(source: leu32) -> u32 {
        arch::from_little_u32(source.0)
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct leu64(u64);

impl From<leu64> for u64 {
    fn from(source: leu64) -> u64 {
        arch::from_little_u64(source.0)
    }
}


#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct beu16(u16);

impl From<beu16> for u16 {
    fn from(source: beu16) -> u16 {
        arch::from_big_u16(source.0)
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct beu32(u32);

impl From<beu32> for u32 {
    fn from(source: beu32) -> u32 {
        arch::from_big_u32(source.0)
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub struct beu64(u64);

impl From<beu64> for u64 {
    fn from(source: beu64) -> u64 {
        arch::from_big_u64(source.0)
    }
}

