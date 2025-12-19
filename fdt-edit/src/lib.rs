#![no_std]

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
