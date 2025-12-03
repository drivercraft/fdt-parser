use core::{
    ffi::FromBytesUntilNulError,
    fmt::{Debug, Display},
};

pub const FDT_MAGIC: u32 = 0xd00dfeed;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Token {
    BeginNode,
    EndNode,
    Prop,
    Nop,
    End,
    Data(u32),
}

impl From<u32> for Token {
    fn from(value: u32) -> Self {
        match value {
            0x1 => Token::BeginNode,
            0x2 => Token::EndNode,
            0x3 => Token::Prop,
            0x4 => Token::Nop,
            0x9 => Token::End,
            _ => Token::Data(value),
        }
    }
}

impl From<Token> for u32 {
    fn from(value: Token) -> Self {
        match value {
            Token::BeginNode => 0x1,
            Token::EndNode => 0x2,
            Token::Prop => 0x3,
            Token::Nop => 0x4,
            Token::End => 0x9,
            Token::Data(v) => v,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Okay,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Phandle(u32);

impl From<u32> for Phandle {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
impl Phandle {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl Display for Phandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<{:#x}>", self.0)
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum FdtError {
    #[error("not found")]
    NotFound,
    #[error("buffer too small at position {pos}")]
    BufferTooSmall { pos: usize },
    #[error("invalid magic number {0:#x} != {FDT_MAGIC:#x}")]
    InvalidMagic(u32),
    #[error("invalid pointer")]
    InvalidPtr,
    #[error("invalid input")]
    InvalidInput,
    #[error("data provided does not contain a nul")]
    FromBytesUntilNull,
    #[error("failed to parse UTF-8 string")]
    Utf8Parse,
    #[error("no aliase `{0}` found")]
    NoAlias(&'static str),
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
