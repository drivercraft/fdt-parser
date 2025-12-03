use core::ops::{Deref, Range};

use crate::define::{FdtError, Token};

#[derive(Clone)]
pub(crate) struct Bytes<'a> {
    all: &'a [u8],
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
}

#[derive(Clone)]
pub(crate) struct Reader<'a> {
    bytes: Bytes<'a>,
    iter: usize,
}

impl<'a> Reader<'a> {
    pub fn position(&self) -> usize {
        self.bytes.range.start + self.iter
    }

    pub fn remain(&self) -> Bytes<'a> {
        self.bytes.slice(self.iter..self.bytes.len())
    }

    pub fn read_bytes(&mut self, size: usize) -> Option<&'a [u8]> {
        if self.iter + size > self.bytes.len() {
            return None;
        }
        let start = self.iter;
        self.iter += size;
        Some(&self.bytes.all[self.bytes.range.start + start..self.bytes.range.start + start + size])
    }

    pub fn read_token(&mut self) -> Result<Token, FdtError> {
        let bytes = self.read_bytes(4).ok_or(FdtError::BufferTooSmall {
            pos: self.position(),
        })?;
        Ok(u32::from_be_bytes(bytes.try_into().unwrap()).into())
    }

    pub fn backtrack(&mut self, size: usize) {
        assert!(size <= self.iter);
        self.iter -= size;
    }
}
