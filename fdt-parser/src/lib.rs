//! A `#![no_std]` Flattened Device Tree (FDT) parser for Rust.
//!
//! This crate provides a pure-Rust parser for Device Tree Blob (DTB) files
//! based on the devicetree-specification-v0.4. It supports both direct parsing
//! and a cached representation for efficient repeated lookups.
//!
//! # Features
//!
//! - `#![no_std]` compatible - suitable for bare-metal and embedded systems
//! - Two parsing modes:
//!   - [`base`] - Direct parsing that walks the FDT structure
//!   - [`cache`] - Cached representation with indexed nodes for faster lookups
//! - Zero-copy parsing where possible
//! - Comprehensive error handling
//!
//! # Example
//!
//! ```no_run
//! use fdt_parser::Fdt;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Read DTB data from file or memory
//! let data = std::fs::read("path/to/device.dtb")?;
//!
//! // Parse the FDT
//! let fdt = Fdt::from_bytes(&data)?;
//!
//! // Get the root node
//! let root = fdt.get_node_by_path("/").unwrap();
//! println!("Root node: {}", root.name());
//!
//! // Iterate over all nodes
//! for node in fdt.all_nodes() {
//!     println!("Node: {}", node.name());
//! }
//! # Ok(())
//! # }
//! ```

#![no_std]
#![deny(warnings, missing_docs)]

extern crate alloc;

/// Macro to unwrap `Option` values, returning `FdtError::NotFound` if `None`.
///
/// # Variants
///
/// - `none_ok!(expr)` - Returns `FdtError::NotFound` if `expr` is `None`
/// - `none_ok!(expr, err)` - Returns the specified error if `expr` is `None`
macro_rules! none_ok {
    ($e:expr) => {{
        let Some(v) = $e else {
            return Err(crate::FdtError::NotFound);
        };
        v
    }};
    ($e:expr, $err:expr) => {{
        let Some(v) = $e else {
            return Err($err);
        };
        v
    }};
}

mod data;
mod define;
mod header;
mod property;

pub mod base;
pub mod cache;

use core::ffi::FromBytesUntilNulError;

pub use cache::*;
pub use define::*;
pub use header::Header;
pub use property::Property;

/// Errors that can occur during FDT parsing and traversal.
#[derive(thiserror::Error, Debug, Clone)]
pub enum FdtError {
    /// A requested item (node, property, etc.) was not found
    #[error("not found")]
    NotFound,
    /// The buffer is too small to contain the expected data at the given position
    #[error("buffer too small at position {pos}")]
    BufferTooSmall {
        /// The position at which the buffer was found to be too small
        pos: usize,
    },
    /// The FDT magic number does not match the expected value
    #[error("invalid magic number {0:#x} != {FDT_MAGIC:#x}")]
    InvalidMagic(u32),
    /// An invalid pointer was encountered during parsing
    #[error("invalid pointer")]
    InvalidPtr,
    /// String data does not contain a null terminator
    #[error("data provided does not contain a nul")]
    FromBytesUntilNull,
    /// Failed to parse data as UTF-8
    #[error("failed to parse UTF-8 string")]
    Utf8Parse,
    /// No alias was found for the requested path
    #[error("no aliase found")]
    NoAlias,
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
    /// Converts a UTF-8 parsing error into `FdtError::Utf8Parse`.
    fn from(_: core::str::Utf8Error) -> Self {
        FdtError::Utf8Parse
    }
}
impl From<FromBytesUntilNulError> for FdtError {
    /// Converts a C-string parsing error into `FdtError::FromBytesUntilNull`.
    fn from(_: FromBytesUntilNulError) -> Self {
        FdtError::FromBytesUntilNull
    }
}
