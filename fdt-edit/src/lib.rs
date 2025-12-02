#![no_std]

extern crate alloc;

mod fdt;
mod node;
mod prop;

pub use fdt::{Fdt, FdtData, MemoryReservation};
pub use node::Node;
pub use prop::{Phandle, Property, RangesEntry, RawProperty, RegEntry, Status};
