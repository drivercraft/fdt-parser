use core::fmt::{Debug, Display};

use crate::data::{Buffer, Raw, U32Iter};

pub const FDT_MAGIC: u32 = 0xd00dfeed;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Token {
    BeginNode,
    EndNode,
    Prop,
    Nop,
    End,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Okay,
    Disabled,
}

#[derive(Clone, Copy)]
pub struct MemoryRegion {
    pub address: *mut u8,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Phandle(u32);

impl From<u32> for Phandle {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl Phandle {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl Display for Phandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<{:#x}>", self.0)
    }
}

#[derive(Clone, Copy)]
pub struct FdtReg {
    /// parent bus address
    pub address: u64,
    /// child bus address
    pub child_bus_address: u64,
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

/// Range mapping child bus addresses to parent bus addresses
#[derive(Clone)]
pub struct FdtRange<'a> {
    data_child: Raw<'a>,
    data_parent: Raw<'a>,
    /// Size of range
    pub size: u64,
}

impl<'a> FdtRange<'a> {
    pub fn child_bus_address(&self) -> U32Iter<'a> {
        U32Iter::new(self.data_child)
    }

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
        reader: Buffer<'a>,
    ) -> Self {
        Self {
            address_cell,
            address_cell_parent,
            size_cell,
            reader,
        }
    }

    pub fn iter(&self) -> FdtRangeIter<'a> {
        FdtRangeIter { s: self.clone() }
    }
}
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
