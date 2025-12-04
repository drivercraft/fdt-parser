#![no_std]

#[macro_use]
extern crate alloc;

mod fdt;
mod node;
mod prop;

use alloc::{string::String, vec::Vec};
pub use fdt::{Fdt, FdtData, MemoryReservation};
pub use node::{Node, NodeOp};
pub use prop::{Phandle, Property, PropertyOp, RangesEntry, RawProperty, RegInfo, Status};

#[derive(Clone, Debug)]
pub struct FdtContext {
    pub parents: Vec<String>,
    pub parent_address_cells: u8,
    pub parent_size_cells: u8,
    pub ranges: Vec<RangesEntry>,
    pub interrupt_parent: Option<Phandle>,
}
