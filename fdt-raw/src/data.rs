use core::{
    ffi::CStr,
    ops::{Deref, Range},
};

use crate::define::{FdtError, Token};

#[derive(Clone)]
pub struct Bytes<'a> {
    pub(crate) all: &'a [u8],
    range: Range<usize>,
}

impl Deref for Bytes<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.all[self.range.clone()]
    }
}

impl<'a> Bytes<'a> {
    pub fn new(all: &'a [u8]) -> Self {
        Self {
            all,
            range: 0..all.len(),
        }
    }
    pub fn slice(&self, range: Range<usize>) -> Self {
        assert!(range.end <= self.len());
        Self {
            all: self.all,
            range: (self.range.start + range.start)..(self.range.start + range.end),
        }
    }

    pub fn as_slice(&self) -> &'a [u8] {
        &self.all[self.range.clone()]
    }

    pub fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    pub fn reader(&self) -> Reader<'a> {
        Reader {
            bytes: self.slice(0..self.len()),
            iter: 0,
        }
    }

    pub fn reader_at(&self, position: usize) -> Reader<'a> {
        assert!(position < self.len());
        Reader {
            bytes: self.slice(position..self.len()),
            iter: 0,
        }
    }

    pub fn as_u32_iter(&self) -> U32Iter<'a> {
        U32Iter {
            reader: self.reader(),
        }
    }

    pub fn as_str_iter(&self) -> StrIter<'a> {
        StrIter {
            reader: self.reader(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone)]
pub struct Reader<'a> {
    pub(crate) bytes: Bytes<'a>,
    pub(crate) iter: usize,
}

impl<'a> Reader<'a> {
    pub fn position(&self) -> usize {
        self.bytes.range.start + self.iter
    }

    pub fn remain(&self) -> Bytes<'a> {
        self.bytes.slice(self.iter..self.bytes.len())
    }

    pub fn read_bytes(&mut self, size: usize) -> Option<Bytes<'a>> {
        if self.iter + size > self.bytes.len() {
            return None;
        }
        let start = self.iter;
        self.iter += size;
        Some(self.bytes.slice(start..start + size))
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        let bytes = self.read_bytes(4)?;
        Some(u32::from_be_bytes(bytes.as_slice().try_into().unwrap()))
    }

    pub fn read_u64(&mut self) -> Option<u64> {
        let high = self.read_u32()? as u64;
        let low = self.read_u32()? as u64;
        Some((high << 32) | low)
    }

    pub fn read_cells(&mut self, cell_count: usize) -> Option<u64> {
        let mut value: u64 = 0;
        for _ in 0..cell_count {
            let cell = self.read_u32()? as u64;
            value = (value << 32) | cell;
        }
        Some(value)
    }

    pub fn read_token(&mut self) -> Result<Token, FdtError> {
        let bytes = self.read_bytes(4).ok_or(FdtError::BufferTooSmall {
            pos: self.position(),
        })?;
        Ok(u32::from_be_bytes(bytes.as_slice().try_into().unwrap()).into())
    }

    pub fn backtrack(&mut self, size: usize) {
        assert!(size <= self.iter);
        self.iter -= size;
    }
}

#[derive(Clone)]
pub struct U32Iter<'a> {
    pub reader: Reader<'a>,
}

impl Iterator for U32Iter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.reader.read_bytes(4)?;
        Some(u32::from_be_bytes(bytes.as_slice().try_into().unwrap()))
    }
}

#[derive(Clone)]
pub struct StrIter<'a> {
    pub reader: Reader<'a>,
}

impl<'a> Iterator for StrIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let remain = self.reader.remain();
        if remain.is_empty() {
            return None;
        }
        let s = CStr::from_bytes_until_nul(remain.as_slice())
            .ok()?
            .to_str()
            .ok()?;
        let str_len = s.len() + 1; // 包括 null 终止符
        self.reader.read_bytes(str_len)?; // 移动读取位置
        Some(s)
    }
}
