
use core::fmt;
use core::fmt::Write;

#[macro_export]
macro_rules! write_bytes {
    ($buffer:expr, $fmt:expr, $($args:tt)*) => {
        {
            let mut writer = $crate::misc::writer::SliceWriter::new(&mut $buffer);
            write!(writer, $fmt, $($args)*).map_err(|_| $crate::errors::KernelError::InvalidArgument)?;
            writer.len()
        }
    }
}

pub struct SliceWriter<'a> {
    slice: &'a mut [u8],
    position: usize,
}

impl<'a> SliceWriter<'a> {
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self {
            slice,
            position: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.position
    }
}

impl<'a> Write for SliceWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut len = s.as_bytes().len();
        if self.position + len > self.slice.len() {
            len = self.slice.len() - self.position;
        }

        (&mut self.slice[self.position..self.position + len]).copy_from_slice(s.as_bytes());
        self.position += len;
        Ok(())
    }
}

