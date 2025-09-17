use core::fmt::Display;

use crate::data::{Raw, U32Iter};

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

#[derive(Debug, Clone)]
pub struct ReserveEntry {
    pub address: u64,
    pub size: u64,
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
