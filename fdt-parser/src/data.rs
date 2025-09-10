use core::ffi::CStr;

use crate::{FdtError, Token};

#[derive(Clone, Copy)]
pub struct Raw<'a> {
    value: &'a [u8],
    pub pos: usize,
}

impl<'a> Raw<'a> {
    pub fn new(value: &'a [u8]) -> Self {
        Raw { value, pos: 0 }
    }

    pub fn buffer_at(&self, offset: usize) -> Buffer<'a> {
        Buffer {
            value: self.value,
            pos: self.pos + offset,
        }
    }

    pub fn all(&self) -> Raw<'a> {
        Raw {
            value: self.value,
            pos: 0,
        }
    }

    pub fn raw(&self) -> &'a [u8] {
        self.value[self.pos..].as_ref()
    }

    pub fn begin_at(&self, offset: usize) -> Result<Raw<'a>, FdtError> {
        let pos = self.pos + offset;
        if pos < self.value.len() {
            Ok(Raw {
                value: self.value,
                pos,
            })
        } else {
            Err(FdtError::BufferTooSmall { pos })
        }
    }
}

pub struct Buffer<'a> {
    value: &'a [u8],
    pos: usize,
}

impl<'a> Buffer<'a> {
    pub fn take(&mut self, size: usize) -> Result<&'a [u8], FdtError> {
        let start = self.pos;
        let end = start + size;
        if end <= self.value.len() {
            self.pos = end;
            Ok(&self.value[start..end])
        } else {
            Err(FdtError::BufferTooSmall { pos: end })
        }
    }

    fn raw(&self) -> Raw<'a> {
        Raw {
            value: self.value,
            pos: self.pos,
        }
    }

    fn remain(&self) -> &'a [u8] {
        &self.value[self.pos..]
    }

    pub fn take_u32(&mut self) -> Result<u32, FdtError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn take_u64(&mut self) -> Result<u64, FdtError> {
        let bytes = self.take(8)?;
        Ok(u64::from_be_bytes(bytes.try_into().unwrap()))
    }

    pub fn take_token(&mut self) -> Result<Token, FdtError> {
        let u = self.take_u32()?;
        Ok(Token::from(u))
    }

    pub fn take_str(&mut self) -> Result<&'a str, FdtError> {
        let remain = self.remain();
        if remain.is_empty() {
            return Err(FdtError::BufferTooSmall { pos: self.pos });
        }

        let cs = CStr::from_bytes_until_nul(remain).map_err(|_| FdtError::FromBytesUntilNull)?;

        let s = cs.to_str().map_err(|_| FdtError::Utf8Parse)?;

        let str_len = cs.to_bytes_with_nul().len();
        // Align to 4-byte boundary for FDT format
        let aligned_len = (str_len + 3) & !3;
        self.pos += aligned_len;

        Ok(s)
    }

    pub fn skip_4_aligned(&mut self, len: usize) -> Result<(), FdtError> {
        self.take((len + 3) & !0x3)?;
        Ok(())
    }
}
