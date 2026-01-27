//! Ranges property parser for address translation.
//!
//! This module provides types for parsing the `ranges` property, which maps
//! child bus addresses to parent bus addresses for address translation.

use crate::data::{Bytes, Reader};

/// Ranges property wrapper for parsing address translation entries.
///
/// The `ranges` property maps child bus address ranges to parent bus address
/// ranges, enabling translation between address spaces.
#[derive(Clone)]
pub struct VecRange<'a> {
    address_cells: usize,
    parent_address_cells: usize,
    size_cells: usize,
    data: Bytes<'a>,
}

impl<'a> VecRange<'a> {
    /// Creates a new VecRange parser.
    pub(crate) fn new(
        address_cells: usize,
        parent_address_cells: usize,
        size_cells: usize,
        data: Bytes<'a>,
    ) -> Self {
        Self {
            address_cells,
            parent_address_cells,
            size_cells,
            data,
        }
    }

    /// Returns an iterator over range entries.
    pub fn iter(&self) -> VecRangeIter<'a> {
        VecRangeIter {
            address_cells: self.address_cells,
            parent_address_cells: self.parent_address_cells,
            size_cells: self.size_cells,
            reader: self.data.reader(),
        }
    }
}

/// Range entry information.
///
/// Represents a single entry in a `ranges` property, mapping a child bus
/// address range to a parent bus address range.
#[derive(Debug, Clone)]
pub struct RangeInfo {
    /// Child bus address
    pub child_address: u64,
    /// Parent bus address
    pub parent_address: u64,
    /// Length of the region
    pub length: u64,
}

/// Iterator over range entries.
pub struct VecRangeIter<'a> {
    address_cells: usize,
    parent_address_cells: usize,
    size_cells: usize,
    reader: Reader<'a>,
}

impl<'a> Iterator for VecRangeIter<'a> {
    type Item = RangeInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let child_address = self.reader.read_cells(self.address_cells)?;
        let parent_address = self.reader.read_cells(self.parent_address_cells)?;
        let length = self.reader.read_cells(self.size_cells)?;

        Some(RangeInfo {
            child_address,
            parent_address,
            length,
        })
    }
}
