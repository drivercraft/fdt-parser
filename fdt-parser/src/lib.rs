#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

mod data;
mod define;
mod header;
mod node;
mod root;
mod walk;

pub use define::*;
pub use header::Header;
pub use node::*;
pub use root::*;
pub use walk::*;

#[derive(thiserror::Error, Debug, Clone)]
pub enum FdtError {
    #[error("buffer too small at position {pos}")]
    BufferTooSmall { pos: usize },
    #[error("invalid magic number {0:#x} != {FDT_MAGIC:#x}")]
    InvalidMagic(u32),
    #[error("invalid pointer")]
    InvalidPtr,
    #[error("invalid UTF-8 string")]
    FromBytesUntilNull,
    #[error("failed to parse UTF-8 string")]
    Utf8Parse,
}
