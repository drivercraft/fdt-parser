#![cfg_attr(not(test), no_std)]
#![doc = include_str!("../README.md")]

mod chosen;
mod clocks;
mod define;
pub mod error;
mod fdt;
mod interrupt;
mod memory;
mod meta;
mod node;
mod pci;
mod property;
mod read;

use define::*;

pub use chosen::Chosen;
pub use clocks::ClockRef;
pub use define::{FdtHeader, MemoryRegion, Phandle};
pub use error::FdtError;
pub use fdt::Fdt;
pub use interrupt::InterruptController;
pub use node::Node;
pub use pci::{Pci, PciRange, PciSpace};
pub use property::Property;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Okay,
    Disabled,
}
