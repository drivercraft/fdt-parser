use core::{
    fmt::{Debug, Display},
    ptr::NonNull,
};

use crate::{
    error::*,
    read::{FdtReader, U32Array},
};

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
    #[inline(always)]
    pub const fn new() -> Self {
        Self([0; 4])
    }

    #[inline(always)]
    pub fn get(self) -> u32 {
        u32::from_be_bytes(self.0)
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
pub struct FdtHeader {
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
    #[inline(always)]
    pub(crate) fn valid_magic(&self) -> FdtResult<'static> {
        if self.magic.get() == 0xd00dfeed {
            Ok(())
        } else {
            Err(FdtError::BadMagic)
        }
    }

    #[inline(always)]
    pub(crate) fn struct_range(&self) -> core::ops::Range<usize> {
        let start = self.off_dt_struct.get() as usize;
        let end = start + self.size_dt_struct.get() as usize;

        start..end
    }

    #[inline(always)]
    pub(crate) fn strings_range(&self) -> core::ops::Range<usize> {
        let start = self.off_dt_strings.get() as usize;
        let end = start + self.size_dt_strings.get() as usize;
        start..end
    }

    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> FdtResult<'static, Self> {
        if bytes.len() < size_of::<FdtHeader>() {
            return Err(FdtError::Eof);
        }

        unsafe {
            let ptr: *const FdtHeader = bytes.as_ptr().cast();
            Ok(ptr.read())
        }
    }

    #[inline(always)]
    pub fn from_ptr(ptr: NonNull<u8>) -> FdtResult<'static, Self> {
        let ptr: NonNull<FdtHeader> = ptr.cast();
        unsafe {
            ptr.as_ref().valid_magic()?;
            Ok(*ptr.as_ref())
        }
    }

    #[inline(always)]
    pub fn total_size(&self) -> usize {
        self.totalsize.get() as _
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
    data_child: &'a [u8],
    data_parent: &'a [u8],
    /// Size of range
    pub size: u64,
}

impl<'a> FdtRange<'a> {
    pub fn child_bus_address(&self) -> U32Array<'a> {
        U32Array::new(self.data_child)
    }

    pub fn parent_bus_address(&self) -> U32Array<'a> {
        U32Array::new(self.data_parent)
    }
}

impl Debug for FdtRange<'_> {
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
        let data_child = self.s.reader.take(child_address_bytes)?;

        let parent_address_bytes = self.s.address_cell_parent as usize * size_of::<u32>();
        let data_parent = self.s.reader.take(parent_address_bytes)?;

        // let child_bus_address = self.s.reader.take_by_cell_size(self.s.address_cell)?;
        // let parent_bus_address = self
        //     .s
        //     .reader
        //     .take_by_cell_size(self.s.address_cell_parent)?;
        let size = self.s.reader.take_by_cell_size(self.s.size_cell)?;
        Some(FdtRange {
            size,
            data_child,
            data_parent,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
