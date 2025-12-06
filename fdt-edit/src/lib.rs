#![no_std]

#[macro_use]
extern crate alloc;

mod ctx;
mod encode;
mod fdt;
mod node;
mod prop;

pub use ctx::FdtContext;
pub use encode::{EncodeContext, FdtData, FdtEncoder, NodeEncode};
pub use fdt::{Fdt, MemoryReservation};
pub use node::{Node, NodeMut, NodeOp, NodeRef, PciRange, PciSpace};
pub use prop::{Phandle, Property, RangesEntry, RawProperty, RegInfo, Status};
