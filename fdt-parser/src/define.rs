//! Common type definitions and constants for FDT parsing.
//!
//! This module defines the core data types, constants, and enumerations
//! used throughout the FDT parser, including the magic number, tokens,
//! status values, and device tree-specific structures.

use core::fmt::{Debug, Display};

use crate::data::{Buffer, Raw, U32Iter};

/// The Device Tree Blob magic number (0xd00dfeed).
///
/// This value must be present at the start of any valid Device Tree Blob.
pub const FDT_MAGIC: u32 = 0xd00dfeed;

/// Token type for parsing FDT structure blocks.
///
/// Tokens are 32-bit values that identify different elements in the
/// device tree structure block.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Token {
    /// Begin node token (0x1)
    BeginNode,
    /// End node token (0x2)
    EndNode,
    /// Property token (0x3)
    Prop,
    /// No-op token (0x4)
    Nop,
    /// End token (0x9) - marks the end of the structure block
    End,
    /// Any other data (not a valid token)
    Data,
}

impl From<u32> for Token {
    fn from(value: u32) -> Self {
        match value {
            0x1 => Token::BeginNode,
            0x2 => Token::EndNode,
            0x3 => Token::Prop,
            0x4 => Token::Nop,
            0x9 => Token::End,
            _ => Token::Data,
        }
    }
}

/// Device node status indicating whether the node is enabled or disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Node is enabled and operational ("okay")
    Okay,
    /// Node is disabled ("disabled")
    Disabled,
}

/// A memory reservation entry in the FDT.
///
/// Memory reservations specify physical memory regions that must
/// not be overwritten by the device tree or bootloader.
#[derive(Clone, Copy)]
pub struct MemoryRegion {
    /// Physical address of the reserved region
    pub address: *mut u8,
    /// Size of the reserved region in bytes
    pub size: usize,
}

impl Debug for MemoryRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "MemoryRegion {{ address: {:p}, size: {:#x} }}",
            self.address, self.size
        ))
    }
}

/// A phandle (pointer handle) for referencing nodes in the device tree.
///
/// Phandles are unique integer identifiers assigned to nodes that need
/// to be referenced from other nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Phandle(u32);

impl From<u32> for Phandle {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl Phandle {
    /// Returns the phandle value as a usize.
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl Display for Phandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<{:#x}>", self.0)
    }
}

/// A register entry describing a memory-mapped region.
///
/// The `reg` property contains one or more of these entries, each
/// describing a address range for a device's registers.
#[derive(Clone, Copy)]
pub struct FdtReg {
    /// Parent bus address
    pub address: u64,
    /// Child bus address
    pub child_bus_address: u64,
    /// Size of the region (None if not specified)
    pub size: Option<usize>,
}

impl Debug for FdtReg {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("<{:#x}", self.address))?;
        if self.child_bus_address != self.address {
            f.write_fmt(format_args!("({:#x})", self.child_bus_address))?;
        }
        f.write_fmt(format_args!(", "))?;
        if let Some(s) = self.size {
            f.write_fmt(format_args!("{:#x}>", s))
        } else {
            f.write_str("None>")
        }
    }
}

/// Range mapping child bus addresses to parent bus addresses.
///
/// The `ranges` property uses these entries to describe how addresses
/// on one bus are translated to another bus.
#[derive(Clone)]
pub struct FdtRange<'a> {
    data_child: Raw<'a>,
    data_parent: Raw<'a>,
    /// Size of range
    pub size: u64,
}

impl<'a> FdtRange<'a> {
    /// Returns an iterator over the child bus address cells.
    pub fn child_bus_address(&self) -> U32Iter<'a> {
        U32Iter::new(self.data_child)
    }

    /// Returns an iterator over the parent bus address cells.
    pub fn parent_bus_address(&self) -> U32Iter<'a> {
        U32Iter::new(self.data_parent)
    }
}

impl core::fmt::Debug for FdtRange<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Range {{ child_bus_address: [ ")?;
        for addr in self.child_bus_address() {
            f.write_fmt(format_args!("{:#x} ", addr))?;
        }
        f.write_str("], parent_bus_address: [ ")?;
        for addr in self.parent_bus_address() {
            f.write_fmt(format_args!("{:#x} ", addr))?;
        }
        f.write_fmt(format_args!("], size: {:#x}", self.size))
    }
}

/// A slice of range entries with associated cell size information.
#[derive(Clone)]
pub struct FdtRangeSilce<'a> {
    address_cell: u8,
    address_cell_parent: u8,
    size_cell: u8,
    reader: Buffer<'a>,
}

impl<'a> FdtRangeSilce<'a> {
    pub(crate) fn new(
        address_cell: u8,
        address_cell_parent: u8,
        size_cell: u8,
        raw: &Raw<'a>,
    ) -> Self {
        Self {
            address_cell,
            address_cell_parent,
            size_cell,
            reader: raw.buffer(),
        }
    }

    /// Returns an iterator over the range entries.
    pub fn iter(&self) -> FdtRangeIter<'a> {
        FdtRangeIter { s: self.clone() }
    }
}

/// Iterator over range entries.
#[derive(Clone)]
pub struct FdtRangeIter<'a> {
    s: FdtRangeSilce<'a>,
}

impl<'a> Iterator for FdtRangeIter<'a> {
    type Item = FdtRange<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let child_address_bytes = self.s.address_cell as usize * size_of::<u32>();
        let data_child = self.s.reader.take(child_address_bytes).ok()?;

        let parent_address_bytes = self.s.address_cell_parent as usize * size_of::<u32>();
        let data_parent = self.s.reader.take(parent_address_bytes).ok()?;

        let size = self.s.reader.take_by_cell_size(self.s.size_cell)?;
        Some(FdtRange {
            size,
            data_child,
            data_parent,
        })
    }
}
