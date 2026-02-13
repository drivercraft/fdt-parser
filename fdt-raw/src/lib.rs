//! Raw FDT parser without high-level abstractions.
//!
//! This crate provides a very low-level parser for Flattened Device Tree (FDT) files.
//! It is designed to be a minimal dependency that only handles the binary format
//! of device tree blobs without providing any node or property abstractions.
//!
//! # Features
//!
//! - `#![no_std]` compatible
//! - Zero-copy parsing where possible
//! - Direct access to the FDT structure blocks
//! - Minimal dependencies
//!
//! # Example
//!
//! ```no_run
//! use fdt_raw::{Fdt, Header};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Read FDT data from file or memory
//! let data = std::fs::read("path/to/device.dtb")?;
//!
//! // Parse the header
//! let header = Header::from_bytes(&data)?;
//!
//! println!("FDT version: {}", header.version);
//! println!("Total size: {} bytes", header.totalsize);
//!
//! // Create the FDT parser
//! let fdt = Fdt::from_bytes(&data)?;
//!
//! // Iterate over memory reservation entries
//! for rsv in fdt.memory_reservations() {
//!     println!("Reserved: {:?} - {:?} bytes", rsv.address, rsv.size);
//! }
//! # Ok(())
//! # }
//! ```

#![no_std]
#![deny(warnings, missing_docs)]

pub mod data;
mod define;
mod fdt;
mod header;
mod iter;
mod node;

mod fmt_utils;

pub use define::*;
pub use fdt::Fdt;
pub use header::Header;
pub use node::*;
