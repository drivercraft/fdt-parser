//! Reg property parser for device register addresses.
//!
//! This module provides types for parsing the `reg` property, which describes
//! memory-mapped registers and address ranges for devices.

use crate::data::Reader;

/// Reg entry information.
///
/// Represents a single entry in a `reg` property, describing an address
/// range for a device's registers or memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegInfo {
    /// Base address
    pub address: u64,
    /// Region size (optional, as size can be 0)
    pub size: Option<u64>,
}

impl RegInfo {
    /// Creates a new RegInfo.
    pub fn new(address: u64, size: Option<u64>) -> Self {
        Self { address, size }
    }
}

/// Reg property iterator.
///
/// Iterates over entries in a `reg` property, parsing address and size
/// values based on the parent node's #address-cells and #size-cells values.
#[derive(Clone)]
pub struct RegIter<'a> {
    reader: Reader<'a>,
    address_cells: u8,
    size_cells: u8,
}

impl<'a> RegIter<'a> {
    /// Creates a new Reg iterator.
    pub(crate) fn new(reader: Reader<'a>, address_cells: u8, size_cells: u8) -> RegIter<'a> {
        RegIter {
            reader,
            address_cells,
            size_cells,
        }
    }
}

impl Iterator for RegIter<'_> {
    type Item = RegInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let address;
        let size;

        // Read address based on address_cells
        if self.address_cells == 1 {
            address = self.reader.read_u32().map(|addr| addr as u64)?;
        } else if self.address_cells == 2 {
            address = self.reader.read_u64()?;
        } else {
            return None;
        }

        // Read size based on size_cells
        if self.size_cells == 0 {
            size = None;
        } else if self.size_cells == 1 {
            size = self.reader.read_u32().map(|s| s as u64);
        } else if self.size_cells == 2 {
            size = self.reader.read_u64();
        } else {
            // Unsupported size_cells value
            return None;
        }

        Some(RegInfo::new(address, size))
    }
}
