#![no_std]

mod define;
pub mod error;
mod fdt;
mod meta;
mod node;
mod property;
mod read;

use define::*;

pub use fdt::Fdt;
