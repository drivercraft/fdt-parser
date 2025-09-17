use core::{
    ffi::CStr,
    ops::{Deref, Range},
};

use crate::{Fdt, FdtError, Property, Token};

#[derive(Clone, Copy)]
pub struct Raw<'a> {
    value: &'a [u8],
    pos: usize,
}

impl<'a> Raw<'a> {
    pub(crate) fn new(value: &'a [u8]) -> Self {
        Raw { value, pos: 0 }
    }

    pub fn buffer(&self) -> Buffer<'a> {
        Buffer { raw: *self, pos: 0 }
    }

    pub fn value(&self) -> &'a [u8] {
        self.value
    }

    pub fn begin_at(&self, offset: usize) -> Raw<'a> {
        let pos = self.pos + offset;
        Raw {
            value: &self.value[offset..],
            pos,
        }
    }

    pub fn get_range(&self, range: Range<usize>) -> Result<Raw<'a>, FdtError> {
        let pos = self.pos + range.start;
        let end = pos + range.len();
        if end <= self.value.len() {
            Ok(Raw {
                value: &self.value[range],
                pos,
            })
        } else {
            Err(FdtError::BufferTooSmall { pos: end })
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn as_ref(&self) -> &'a [u8] {
        self.value
    }
}

impl<'a> Deref for Raw<'a> {
    type Target = &'a [u8];

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub struct Buffer<'a> {
    raw: Raw<'a>,
    pos: usize,
}

impl<'a> Buffer<'a> {
    pub fn take(&mut self, size: usize) -> Result<Raw<'a>, FdtError> {
        let start = self.pos;
        let end = start + size;
        if end <= self.raw.value.len() {
            self.pos = end;
            Ok(Raw {
                value: &self.raw.value[start..end],
                pos: self.raw.pos + start,
            })
        } else {
            Err(FdtError::BufferTooSmall {
                pos: self.raw.pos + end,
            })
        }
    }

    pub fn raw(&self) -> Raw<'a> {
        self.raw
    }

    pub fn remain(&self) -> Raw<'a> {
        Raw {
            value: &self.raw.value[self.pos..],
            pos: self.raw.pos + self.pos,
        }
    }

    pub fn take_u32(&mut self) -> Result<u32, FdtError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes(bytes.as_ref().try_into().unwrap()))
    }

    pub fn take_u64(&mut self) -> Result<u64, FdtError> {
        let bytes = self.take(8)?;
        Ok(u64::from_be_bytes(bytes.as_ref().try_into().unwrap()))
    }

    pub(crate) fn take_token(&mut self) -> Result<Token, FdtError> {
        let u = self.take_u32()?;
        Ok(Token::from(u))
    }

    pub fn take_str(&mut self) -> Result<&'a str, FdtError> {
        let remain = self.remain();
        if remain.is_empty() {
            return Err(FdtError::BufferTooSmall { pos: self.pos });
        }

        let cs = CStr::from_bytes_until_nul(remain.as_ref())
            .map_err(|_| FdtError::FromBytesUntilNull)?;

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

    pub fn take_aligned(&mut self, len: usize) -> Result<Raw<'a>, FdtError> {
        let bytes = (len + 3) & !0x3;
        self.take(bytes)
    }

    pub fn take_by_cell_size(&mut self, cell_size: u8) -> Option<u64> {
        match cell_size {
            1 => self.take_u32().map(|s| s as _).ok(),
            2 => self.take_u64().ok(),
            _ => panic!("invalid cell size {}", cell_size),
        }
    }

    pub fn take_prop(&mut self, fdt: &Fdt<'a>) -> Option<Property<'a>> {
        let len = self.take_u32().ok()?;
        let nameoff = self.take_u32().ok()?;
        let data = self.take_aligned(len as _).ok()?;
        Some(Property {
            name: fdt.get_str(nameoff as _).unwrap_or("<error>"),
            data,
        })
    }
}

pub struct U32Iter<'a> {
    buffer: Buffer<'a>,
}

impl<'a> U32Iter<'a> {
    pub fn new(raw: Raw<'a>) -> Self {
        Self {
            buffer: raw.buffer(),
        }
    }
}

impl<'a> Iterator for U32Iter<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.take_u32().ok()
    }
}
