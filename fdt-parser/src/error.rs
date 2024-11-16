pub type FdtResult<'a, T = ()> = Result<T, FdtError<'a>>;

#[derive(Debug)]
pub enum FdtError<'a> {
    NotFound(&'static str),
    /// The FDT had an invalid magic value.
    BadMagic,
    /// The given pointer was null.
    BadPtr,
    /// Invalid cell encoding.
    BadCell,
    /// Unsupported cell size.
    BadCellSize(usize),

    /// The slice passed in was too small to fit the given total size of the FDT
    /// structure.
    Eof,

    MissingProperty,

    Utf8Parse {
        data: &'a [u8],
    },

    FromBytesUntilNull {
        data: &'a [u8],
    },
}
