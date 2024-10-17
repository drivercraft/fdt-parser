use core::{
    fmt::{Debug, Display},
    ptr::NonNull,
};

use crate::{error::*, read::FdtReader};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Token {
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

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Fdt32([u8; 4]);

impl Fdt32 {
    pub const fn new() -> Self {
        Self([0; 4])
    }

    pub fn get(self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self(bytes.get(..4)?.try_into().ok()?))
    }
}

impl From<&[u8]> for Fdt32 {
    fn from(value: &[u8]) -> Self {
        Fdt32(value.get(..4).unwrap().try_into().unwrap())
    }
}

impl Default for Fdt32 {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Fdt64([u8; 8]);

impl Fdt64 {
    pub const fn new() -> Self {
        Self([0; 8])
    }

    pub fn get(&self) -> u64 {
        u64::from_be_bytes(self.0)
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self(bytes.get(..8)?.try_into().ok()?))
    }
}
impl From<&[u8]> for Fdt64 {
    fn from(value: &[u8]) -> Self {
        Self(value.get(..8).unwrap().try_into().unwrap())
    }
}
impl Default for Fdt64 {
    fn default() -> Self {
        Self::new()
    }
}

/// A raw `reg` property value set
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawReg<'a> {
    /// Big-endian encoded bytes making up the address portion of the property.
    /// Length will always be a multiple of 4 bytes.
    pub address: &'a [u8],
    /// Big-endian encoded bytes making up the size portion of the property.
    /// Length will always be a multiple of 4 bytes.
    pub size: &'a [u8],
}
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct FdtHeader {
    /// FDT header magic
    pub magic: Fdt32,
    /// Total size in bytes of the FDT structure
    pub totalsize: Fdt32,
    /// Offset in bytes from the start of the header to the structure block
    pub off_dt_struct: Fdt32,
    /// Offset in bytes from the start of the header to the strings block
    pub off_dt_strings: Fdt32,
    /// Offset in bytes from the start of the header to the memory reservation
    /// block
    pub off_mem_rsvmap: Fdt32,
    /// FDT version
    pub version: Fdt32,
    /// Last compatible FDT version
    pub last_comp_version: Fdt32,
    /// System boot CPU ID
    pub boot_cpuid_phys: Fdt32,
    /// Length in bytes of the strings block
    pub size_dt_strings: Fdt32,
    /// Length in bytes of the struct block
    pub size_dt_struct: Fdt32,
}

impl FdtHeader {
    pub(crate) fn valid_magic(&self) -> FdtResult {
        if self.magic.get() == 0xd00dfeed {
            Ok(())
        } else {
            Err(FdtError::BadMagic)
        }
    }

    pub(crate) fn struct_range(&self) -> core::ops::Range<usize> {
        let start = self.off_dt_struct.get() as usize;
        let end = start + self.size_dt_struct.get() as usize;

        start..end
    }

    pub(crate) fn strings_range(&self) -> core::ops::Range<usize> {
        let start = self.off_dt_strings.get() as usize;
        let end = start + self.size_dt_strings.get() as usize;
        start..end
    }

    pub fn from_bytes(bytes: &[u8]) -> FdtResult<Self> {
        if bytes.len() < size_of::<FdtHeader>() {
            return Err(FdtError::BufferTooSmall);
        }

        unsafe {
            let ptr: *const FdtHeader = bytes.as_ptr().cast();
            Ok(ptr.read())
        }
    }

    pub fn from_ptr(ptr: NonNull<u8>) -> FdtResult<Self> {
        let ptr: NonNull<FdtHeader> = ptr.cast();
        unsafe { Ok(ptr.as_ref().clone()) }
    }
}

impl Display for FdtHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FdtHeader")
            .field("size", &self.totalsize.get())
            .field("version", &self.version.get())
            .field("last_comp_version", &self.last_comp_version.get())
            .finish()
    }
}

#[repr(C)]
pub(crate) struct FdtReserveEntry {
    pub address: u64,
    pub size: u64,
}
impl FdtReserveEntry {
    pub fn new(address: u64, size: u64) -> Self {
        Self { address, size }
    }
}

impl From<FdtReserveEntry> for MemoryRegion {
    fn from(value: FdtReserveEntry) -> Self {
        Self {
            address: value.address as usize as _,
            size: value.size as _,
        }
    }
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

#[derive(Clone, Copy)]
pub struct FdtReg {
    /// parent bus address
    pub address: u128,
    /// child bus address
    pub child_bus_address: u128,
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
#[derive(Clone, Copy, PartialEq)]
pub struct FdtRange {
    /// Starting address on child bus
    pub child_bus_address: u128,
    /// Starting address on parent bus
    pub parent_bus_address: u128,
    /// Size of range
    pub size: usize,
}

impl Debug for FdtRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "Range {{ child_bus_address: {:#x}, parent_bus_address: {:#x}, size: {:#x} }}",
            self.child_bus_address, self.parent_bus_address, self.size
        ))
    }
}

#[derive(Clone)]
pub struct FdtRangeSilce<'a> {
    address_cell: u8,
    address_cell_parent: u8,
    size_cell: u8,
    reader: FdtReader<'a>,
}

impl<'a> FdtRangeSilce<'a> {
    pub(crate) fn new(
        address_cell: u8,
        address_cell_parent: u8,
        size_cell: u8,
        reader: FdtReader<'a>,
    ) -> Self {
        Self {
            address_cell,
            address_cell_parent,
            size_cell,
            reader,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = FdtRange> + 'a {
        FdtRangeIter { s: self.clone() }
    }
}
#[derive(Clone)]
struct FdtRangeIter<'a> {
    s: FdtRangeSilce<'a>,
}

impl<'a> Iterator for FdtRangeIter<'a> {
    type Item = FdtRange;

    fn next(&mut self) -> Option<Self::Item> {
        let child_bus_address = self.s.reader.take_by_cell_size(self.s.address_cell)?;
        let parent_bus_address = self
            .s
            .reader
            .take_by_cell_size(self.s.address_cell_parent)?;
        let size = self.s.reader.take_by_cell_size(self.s.size_cell)? as usize;
        Some(FdtRange {
            child_bus_address,
            parent_bus_address,
            size,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Phandle(u32);

impl From<u32> for Phandle {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Display for Phandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<{:#x}>", self.0)
    }
}
