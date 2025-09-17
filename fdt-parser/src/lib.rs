#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

mod data;
mod define;
mod header;
mod node;
mod property;
mod root;

use core::ffi::FromBytesUntilNulError;

pub use define::*;
pub use header::Header;
pub use node::*;
pub use property::Property;
pub use root::*;

#[derive(thiserror::Error, Debug, Clone)]
pub enum FdtError {
    #[error("buffer too small at position {pos}")]
    BufferTooSmall { pos: usize },
    #[error("invalid magic number {0:#x} != {FDT_MAGIC:#x}")]
    InvalidMagic(u32),
    #[error("invalid pointer")]
    InvalidPtr,
    #[error("data provided does not contain a nul")]
    FromBytesUntilNull,
    #[error("failed to parse UTF-8 string")]
    Utf8Parse,
}

impl From<core::str::Utf8Error> for FdtError {
    fn from(_: core::str::Utf8Error) -> Self {
        FdtError::Utf8Parse
    }
}
impl From<FromBytesUntilNulError> for FdtError {
    fn from(_: FromBytesUntilNulError) -> Self {
        FdtError::FromBytesUntilNull
    }
}
