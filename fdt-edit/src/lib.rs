#![no_std]

#[macro_use]
extern crate alloc;

mod ctx;
mod fdt;
mod node;
mod prop;

pub use ctx::FdtContext;
pub use fdt::{Fdt, FdtData, MemoryReservation};
pub use node::{Node, NodeMut, NodeOp, NodeRef};
pub use prop::{Phandle, Property, RangesEntry, RawProperty, RegInfo, Status};
