#![no_std]

#[macro_use]
extern crate alloc;

mod fdt;
mod node;
mod prop;

pub use fdt::{Fdt, FdtData, MemoryReservation};
pub use node::{Node, NodeOp};
pub use prop::{Phandle, Property, RangesEntry, RawProperty, RegInfo, Status};
