#![no_std]

#[macro_use]
extern crate alloc;

mod fdt;
mod node;
mod node_iter;
mod prop;

pub use fdt_raw::{FdtError, MemoryRegion, Phandle, RegInfo, Status, data::Reader};

pub use fdt::*;
pub use node::*;
pub use node_iter::*;
pub use prop::*;
