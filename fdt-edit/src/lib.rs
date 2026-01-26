//! Device Tree Blob (DTB) editing and manipulation library.
//!
//! This crate provides functionality for creating, modifying, and encoding
//! Flattened Device Tree (FDT) structures. Unlike the parser crates which
//! focus on reading existing device trees, this crate allows you to build
//! and modify device trees programmatically.
//!
//! # Features
//!
//! - `#![no_std]` compatible
//! - Build device trees from scratch
//! - Modify existing device trees
//! - Add/remove nodes and properties
//! - Encode to standard DTB format
//! - Support for overlays
//!
//! # Example
//!
//! ```ignore
//! use fdt_edit::{Fdt, Context, Property, NodeKind};
//!
//! // Create a new FDT with a context
//! let mut fdt = Fdt::new(&Context::default());
//!
//! // Add a root node
//! let root = fdt.root_mut();
//!
//! // Add a memory node
//! let memory = fdt.add_node(
//!     root,
//!     "memory",
//!     NodeKind::Memory
//! );
//!
//! // Add properties to the memory node
//! fdt.add_property(memory, "reg", Property::Reg(&[
//!     RegInfo { address: 0x80000000, size: 0x10000000 },
//! ]));
//!
//! // Encode to DTB format
//! let dtb_data = fdt.encode()?;
//! ```

#![no_std]
#![deny(warnings, missing_docs)]

#[macro_use]
extern crate alloc;

mod ctx;
mod encode;
mod fdt;
mod node;
mod prop;

pub use ctx::Context;
pub use encode::FdtData;
pub use fdt::{Fdt, MemoryReservation};
pub use node::NodeKind;
pub use node::*;
pub use prop::{Phandle, Property, RangesEntry, RegInfo, Status};
