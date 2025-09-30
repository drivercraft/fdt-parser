#![no_std]

extern crate alloc;

macro_rules! none_ok {
    ($e:expr) => {{
        let Some(v) = $e else {
            return Ok(None);
        };
        v
    }};
}

mod data;
mod define;
mod header;
mod property;

pub mod base;
pub mod cache;

use core::ffi::FromBytesUntilNulError;

pub use cache::*;
pub use define::*;
pub use header::Header;
pub use property::Property;

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
    #[error("no aliase found")]
    NoAlias,
    #[error("system out of memory")]
    NoMemory,
    #[error("node `{0}` not found")]
    NodeNotFound(&'static str),
    #[error("property `{0}` not found")]
    PropertyNotFound(&'static str),
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
