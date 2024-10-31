#![no_std]
#![doc = include_str!("../README.md")]

mod chosen;
mod clocks;
mod define;
pub mod error;
mod fdt;
mod interrupt;
mod meta;
mod node;
mod property;
mod read;

use define::*;

pub use chosen::Chosen;
pub use clocks::ClockRef;
pub use define::FdtHeader;
pub use error::FdtError;
pub use fdt::Fdt;
pub use interrupt::{InterruptController, InterruptInfo};
pub use node::Node;
pub use property::Property;
