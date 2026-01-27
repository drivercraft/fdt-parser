//! Direct parsing module for FDT structures.
//!
//! This module provides a zero-copy parser that walks the FDT structure
//! directly without building an in-memory index. It is suitable for
//! one-pass operations where memory efficiency is important.

mod fdt;
mod node;

pub use fdt::*;
pub use node::*;
