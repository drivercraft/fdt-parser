#![no_std]

#[macro_use]
extern crate alloc;

mod fdt;
mod node;
mod prop;
mod view;
mod visit;

pub use fdt_raw::{FdtError, MemoryRegion, Phandle, RegInfo, Status, data::Reader};

/// A unique identifier for a node in the `Fdt` arena.
pub type NodeId = usize;

pub use fdt::*;
pub use node::*;
pub use prop::*;
pub use view::*;
pub use visit::*;
