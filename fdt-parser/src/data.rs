use core::{
    ffi::CStr,
    ops::{Deref, Range},
};

use crate::{base::Fdt, FdtError, Property, Token};

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
        Buffer {
            raw: *self,
            iter: 0,
        }
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

#[derive(Clone)]
pub struct Buffer<'a> {
    raw: Raw<'a>,
    iter: usize,
}

impl<'a> Buffer<'a> {
    pub fn take(&mut self, size: usize) -> Result<Raw<'a>, FdtError> {
        let start = self.iter;
        let end = start + size;
        if end <= self.raw.value.len() {
            self.iter = end;
            Ok(Raw {
                value: &self.raw.value[start..end],
                pos: self.pos(),
            })
        } else {
            Err(FdtError::BufferTooSmall {
                pos: self.pos() + size,
            })
        }
    }

    fn pos(&self) -> usize {
        self.raw.pos + self.iter
    }

    pub fn remain(&self) -> Raw<'a> {
        Raw {
            value: &self.raw.value[self.iter..],
            pos: self.pos(),
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
            return Err(FdtError::BufferTooSmall { pos: self.iter });
        }

        let cs = CStr::from_bytes_until_nul(remain.as_ref())
            .map_err(|_| FdtError::FromBytesUntilNull)?;

        let s = cs.to_str()?;

        let str_len = cs.to_bytes_with_nul().len();
        // Align to 4-byte boundary for FDT format
        // let aligned_len = (str_len + 3) & !3;
        self.iter += str_len;

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

    pub fn take_to_aligned(&mut self) {
        let remain = self.iter % 4;
        if remain != 0 {
            let add = 4 - remain;
            if self.iter + add <= self.raw.value.len() {
                self.iter += 4 - remain;
            } else {
                self.iter = self.raw.value.len();
            }
        }
    }

    pub fn take_by_cell_size(&mut self, cell_size: u8) -> Option<u64> {
        match cell_size {
            1 => self.take_u32().map(|s| s as _).ok(),
            2 => self.take_u64().ok(),
            _ => panic!("invalid cell size {}", cell_size),
        }
    }

    pub fn take_prop(&mut self, fdt: &Fdt<'a>) -> Result<Property<'a>, FdtError> {
        let len = self.take_u32()?;
        let nameoff = self.take_u32()?;
        let data = self.take_aligned(len as _)?;
        Ok(Property {
            name: fdt.get_str(nameoff as _)?,
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

    pub fn as_u64(&mut self) -> u64 {
        let h = self.buffer.take_u32().unwrap();
        if let Ok(l) = self.buffer.take_u32() {
            ((h as u64) << 32) + l as u64
        } else {
            h as _
        }
    }
}

impl<'a> Iterator for U32Iter<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.take_u32().ok()
    }
}

pub struct U32Iter2D<'a> {
    reader: Buffer<'a>,
    row_len: u8,
}

impl<'a> U32Iter2D<'a> {
    pub fn new(bytes: &Raw<'a>, row_len: u8) -> Self {
        Self {
            reader: bytes.buffer(),
            row_len,
        }
    }
}

impl<'a> Iterator for U32Iter2D<'a> {
    type Item = U32Iter<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self
            .reader
            .take(self.row_len as usize * size_of::<u32>())
            .ok()?;
        Some(U32Iter::new(bytes))
    }
}
