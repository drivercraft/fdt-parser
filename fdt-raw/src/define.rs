//! Core type definitions and constants for FDT parsing.
//!
//! This module provides fundamental types used throughout the FDT parser,
//! including the magic number constant, tokens for parsing the structure
//! block, error types, and common enums.

use core::{
    ffi::FromBytesUntilNulError,
    fmt::{Debug, Display},
    ops::Deref,
};

/// The magic number that identifies a valid Flattened Device Tree blob.
///
/// This value (0xd00dfeed) must be present at the beginning of any
/// valid device tree blob. It is used for validation when parsing.
pub const FDT_MAGIC: u32 = 0xd00dfeed;

/// Entry in the memory reservation block.
///
/// The memory reservation block contains a list of physical memory regions
/// that must be preserved (not used by the OS) during boot. Each entry
/// specifies the starting address and size of a reserved region.
#[derive(Clone, Debug, Default)]
pub struct MemoryReservation {
    /// Physical address of the reserved region
    pub address: u64,
    /// Size of the reserved region in bytes
    pub size: u64,
}

/// Token type for parsing the FDT structure block.
///
/// The device tree structure block is composed of a sequence of 32-bit
/// tokens followed by data. This enum represents the possible token values.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Token {
    /// Marks the beginning of a node (FDT_BEGIN_NODE, 0x00000001)
    BeginNode,
    /// Marks the end of a node (FDT_END_NODE, 0x00000002)
    EndNode,
    /// Marks a property (FDT_PROP, 0x00000003)
    Prop,
    /// No-op token, should be ignored (FDT_NOP, 0x00000004)
    Nop,
    /// Marks the end of the structure block (FDT_END, 0x00000009)
    End,
    /// Any other 32-bit value (invalid or unknown token)
    Data(u32),
}

impl From<u32> for Token {
    fn from(value: u32) -> Self {
        match value {
            0x1 => Token::BeginNode,
            0x2 => Token::EndNode,
            0x3 => Token::Prop,
            0x4 => Token::Nop,
            0x9 => Token::End,
            _ => Token::Data(value),
        }
    }
}

impl From<Token> for u32 {
    fn from(value: Token) -> Self {
        match value {
            Token::BeginNode => 0x1,
            Token::EndNode => 0x2,
            Token::Prop => 0x3,
            Token::Nop => 0x4,
            Token::End => 0x9,
            Token::Data(v) => v,
        }
    }
}

/// Device tree node status property value.
///
/// The `status` property in a device tree indicates whether a node is
/// enabled or disabled. A disabled node should generally be ignored by
/// the OS, though the node may still be probed if explicitly requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Node is operational and should be used ("okay")
    Okay,
    /// Node is disabled and should not be used ("disabled")
    Disabled,
}

impl Deref for Status {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Status::Okay => "okay",
            Status::Disabled => "disabled",
        }
    }
}

impl core::fmt::Display for Status {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.deref())
    }
}

/// A phandle (pointer handle) for referencing device tree nodes.
///
/// Phandles provide a way for nodes to reference other nodes in the device tree.
/// A node that may be referenced defines a `phandle` property with a unique value,
/// and other nodes reference it using that value in properties like `interrupt-parent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Phandle(u32);

impl From<u32> for Phandle {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Phandle {
    /// Returns the phandle value as a `usize`.
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }

    /// Returns the raw u32 value of this phandle.
    pub fn raw(&self) -> u32 {
        self.0
    }
}

impl Display for Phandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<{:#x}>", self.0)
    }
}

/// Errors that can occur during FDT parsing.
///
/// This enum represents all possible error conditions that may be encountered
/// when parsing a device tree blob or accessing its contents.
#[derive(thiserror::Error, Debug, Clone)]
pub enum FdtError {
    /// A requested item (node, property, etc.) was not found
    #[error("not found")]
    NotFound,
    /// The buffer is too small to read the requested data at the given position
    #[error("buffer too small at position {pos}")]
    BufferTooSmall {
        /// The position at which the buffer was too small
        pos: usize
    },
    /// The FDT magic number doesn't match the expected value
    #[error("invalid magic number {0:#x} != {FDT_MAGIC:#x}")]
    InvalidMagic(u32),
    /// An invalid pointer was provided
    #[error("invalid pointer")]
    InvalidPtr,
    /// The input data is invalid or malformed
    #[error("invalid input")]
    InvalidInput,
    /// A null-terminated string was expected but not found
    #[error("data provided does not contain a nul")]
    FromBytesUntilNull,
    /// Failed to parse data as a UTF-8 string
    #[error("failed to parse UTF-8 string")]
    Utf8Parse,
    /// The specified alias was not found in the /aliases node
    #[error("no aliase `{0}` found")]
    NoAlias(&'static str),
    /// Memory allocation failed
    #[error("system out of memory")]
    NoMemory,
    /// The specified node was not found
    #[error("node `{0}` not found")]
    NodeNotFound(&'static str),
    /// The specified property was not found
    #[error("property `{0}` not found")]
    PropertyNotFound(&'static str),
}

impl From<core::str::Utf8Error> for FdtError {
    fn from(_: core::str::Utf8Error) -> Self {
        FdtError::Utf8Parse
    }
}
impl From<FromBytesUntilNulError> for FdtError {
    fn from(_: FromBytesUntilNulError) -> Self {
        FdtError::FromBytesUntilNull
    }
}
