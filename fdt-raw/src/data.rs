//! Low-level data access primitives for FDT parsing.
//!
//! This module provides raw data access types for reading FDT binary format,
//! including bytes slices, readers, and various iterators.

use core::{
    ffi::CStr,
    ops::{Deref, Range},
};

use crate::define::{FdtError, Token};

/// Size of a 32-bit (4-byte) value in bytes.
/// Used frequently in FDT parsing for alignment and value sizes.
pub(crate) const U32_SIZE: usize = 4;

/// Memory reservation entry size in bytes (address + size, each 8 bytes).
pub(crate) const MEM_RSV_ENTRY_SIZE: usize = 16;

/// A view into a byte slice with a specific range.
///
/// `Bytes` provides a window into FDT data with range tracking and
/// convenience methods for creating readers and iterators. This allows
/// zero-copy parsing by maintaining references to the original data.
///
/// # Examples
///
/// ```ignore
/// let bytes = Bytes::new(&data);
/// let slice = bytes.slice(10..20);
/// let reader = slice.reader();
/// ```
#[derive(Clone)]
pub struct Bytes<'a> {
    /// Reference to the complete original data buffer
    pub(crate) all: &'a [u8],
    /// The active range within the original buffer
    range: Range<usize>,
}

impl Deref for Bytes<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.all[self.range.clone()]
    }
}

impl<'a> Bytes<'a> {
    /// Creates a new `Bytes` from the entire byte slice.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let bytes = Bytes::new(&my_data);
    /// ```
    pub fn new(all: &'a [u8]) -> Self {
        Self {
            all,
            range: 0..all.len(),
        }
    }

    /// Creates a new `Bytes` from a subrange of the current data.
    ///
    /// # Panics
    ///
    /// Panics if `range.end` exceeds the current length.
    pub fn slice(&self, range: Range<usize>) -> Self {
        assert!(range.end <= self.len());
        Self {
            all: self.all,
            range: (self.range.start + range.start)..(self.range.start + range.end),
        }
    }

    /// Returns the underlying byte slice as reference.
    pub fn as_slice(&self) -> &'a [u8] {
        &self.all[self.range.clone()]
    }

    /// Returns the length of the byte slice.
    pub fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    /// Creates a reader for sequential reading from this position.
    pub fn reader(&self) -> Reader<'a> {
        Reader {
            bytes: self.slice(0..self.len()),
            iter: 0,
        }
    }

    /// Creates a reader starting at a specific position.
    ///
    /// # Panics
    ///
    /// Panics if `position` is greater than or equal to the current length.
    pub fn reader_at(&self, position: usize) -> Reader<'a> {
        assert!(position < self.len());
        Reader {
            bytes: self.slice(position..self.len()),
            iter: 0,
        }
    }

    /// Creates a u32 iterator over this data.
    pub fn as_u32_iter(&self) -> U32Iter<'a> {
        U32Iter {
            reader: self.reader(),
        }
    }

    /// Creates a string iterator over this data.
    pub fn as_str_iter(&self) -> StrIter<'a> {
        StrIter {
            reader: self.reader(),
        }
    }

    /// Checks if the byte slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Sequential reader for parsing FDT data structures.
///
/// `Reader` provides sequential read access with position tracking for
/// parsing FDT binary format. It maintains a current position and can
/// backtrack if needed.
///
/// # Examples
///
/// ```ignore
/// let mut reader = bytes.reader();
/// let value = reader.read_u32()?;
/// let str = reader.read_cstr()?;
/// ```
#[derive(Clone)]
pub struct Reader<'a> {
    /// The byte slice being read from
    pub(crate) bytes: Bytes<'a>,
    /// Current read position within the bytes
    pub(crate) iter: usize,
}

impl<'a> Reader<'a> {
    /// Returns the current read position in the original data.
    pub fn position(&self) -> usize {
        self.bytes.range.start + self.iter
    }

    /// Returns the remaining unread data as a `Bytes`.
    pub fn remain(&self) -> Bytes<'a> {
        self.bytes.slice(self.iter..self.bytes.len())
    }

    /// Reads the specified number of bytes, advancing the position.
    ///
    /// Returns `None` if insufficient bytes remain.
    pub fn read_bytes(&mut self, size: usize) -> Option<Bytes<'a>> {
        if self.iter + size > self.bytes.len() {
            return None;
        }
        let start = self.iter;
        self.iter += size;
        Some(self.bytes.slice(start..start + size))
    }

    /// Reads a big-endian u32 value.
    pub fn read_u32(&mut self) -> Option<u32> {
        let bytes = self.read_bytes(U32_SIZE)?;
        Some(u32::from_be_bytes(bytes.as_slice().try_into().unwrap()))
    }

    /// Reads a big-endian u64 value (composed of two u32 values).
    pub fn read_u64(&mut self) -> Option<u64> {
        let high = self.read_u32()? as u64;
        let low = self.read_u32()? as u64;
        Some((high << 32) | low)
    }

    /// Reads a value composed of the specified number of cells.
    ///
    /// Each cell is 4 bytes (a u32). The cells are combined into a u64 value.
    pub fn read_cells(&mut self, cell_count: usize) -> Option<u64> {
        let mut value: u64 = 0;
        for _ in 0..cell_count {
            let cell = self.read_u32()? as u64;
            value = (value << 32) | cell;
        }
        Some(value)
    }

    /// Reads a token from the FDT structure block.
    pub fn read_token(&mut self) -> Result<Token, FdtError> {
        let bytes = self.read_bytes(U32_SIZE).ok_or(FdtError::BufferTooSmall {
            pos: self.position(),
        })?;
        Ok(u32::from_be_bytes(bytes.as_slice().try_into().unwrap()).into())
    }

    /// Moves the read position back by the specified size.
    pub fn backtrack(&mut self, size: usize) {
        assert!(size <= self.iter);
        self.iter -= size;
    }
}

/// Iterator over u32 values in FDT data.
///
/// Reads big-endian u32 values sequentially from the underlying data.
/// Each iteration consumes 4 bytes.
///
/// # Examples
///
/// ```ignore
/// let mut iter = bytes.as_u32_iter();
/// while let Some(value) = iter.next() {
///     println!("Value: {:#x}", value);
/// }
/// ```
#[derive(Clone)]
pub struct U32Iter<'a> {
    /// The underlying reader for accessing FDT data
    pub reader: Reader<'a>,
}

impl Iterator for U32Iter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.reader.read_bytes(U32_SIZE)?;
        Some(u32::from_be_bytes(bytes.as_slice().try_into().unwrap()))
    }
}

/// Iterator over null-terminated strings in FDT data.
///
/// Reads null-terminated (C-style) strings sequentially from the underlying data.
/// Each iteration consumes the string content plus its null terminator.
///
/// # Examples
///
/// ```ignore
/// let mut iter = bytes.as_str_iter();
/// while let Some(s) = iter.next() {
///     println!("String: {}", s);
/// }
/// ```
#[derive(Clone)]
pub struct StrIter<'a> {
    /// The underlying reader for accessing FDT data
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
        let str_len = s.len() + 1; // including null terminator
        self.reader.read_bytes(str_len)?; // advance read position
        Some(s)
    }
}
