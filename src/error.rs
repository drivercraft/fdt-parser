use core::str::Utf8Error;

pub type FdtResult<T = ()> = Result<T, FdtError>;

#[derive(Debug)]
pub enum FdtError {
    /// The FDT had an invalid magic value.
    BadMagic,
    /// The given pointer was null.
    BadPtr,
    /// Invalid cell encoding.
    BadCell,
    /// Unsupported cell size.
    BadCellSize(usize),
    /// Bad property tag.
    BadPropTag((u32, u32)),
    /// The slice passed in was too small to fit the given total size of the FDT
    /// structure.
    BufferTooSmall,

    MissingProperty,

    Utf8Parse,
}

impl From<Utf8Error> for FdtError {
    fn from(_value: Utf8Error) -> Self {
        FdtError::Utf8Parse
    }
}
