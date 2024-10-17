use core::str::Utf8Error;

pub type FdtResult<'a, T = ()> = Result<T, FdtError<'a>>;

#[derive(Debug)]
pub enum FdtError<'a> {
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

    FromBytesUntilNull {
        data: &'a [u8],
    },
}

impl<'a> From<Utf8Error> for FdtError<'a> {
    fn from(_value: Utf8Error) -> Self {
        FdtError::Utf8Parse
    }
}
