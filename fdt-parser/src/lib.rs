#![no_std]
#![doc = include_str!("../README.md")]

mod chosen;
mod define;
pub mod error;
mod fdt;
mod interrupt;
mod meta;
mod node;
mod property;
mod read;
mod clocks;

use define::*;

pub use define::FdtHeader;
pub use fdt::Fdt;
pub use interrupt::InterruptInfo;