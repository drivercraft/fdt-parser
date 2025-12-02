use crate::{FdtError, NodeIter, data::Bytes, header::Header, iter::FdtIter};

#[derive(Clone)]
pub struct Fdt<'a> {
    header: Header,
    pub(crate) data: Bytes<'a>,
}

impl<'a> Fdt<'a> {
    /// Create a new `Fdt` from byte slice.
    pub fn from_bytes(data: &'a [u8]) -> Result<Fdt<'a>, FdtError> {
        let header = Header::from_bytes(data)?;
        if data.len() < header.totalsize as usize {
            return Err(FdtError::BufferTooSmall {
                pos: header.totalsize as usize,
            });
        }
        let buffer = Bytes::new(data);
        Ok(Fdt {
            header,
            data: buffer,
        })
    }

    /// Create a new `Fdt` from a raw pointer and size in bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// memory region of at least `size` bytes that contains a valid device tree
    /// blob.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Fdt<'a>, FdtError> {
        let header = unsafe { Header::from_ptr(ptr)? };

        let data = Bytes::new(unsafe { core::slice::from_raw_parts(ptr, header.totalsize as _) });

        Ok(Fdt { header, data })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn as_slice(&self) -> &'a [u8] {
        self.data.as_slice()
    }

    pub fn all_nodes(&self) -> FdtIter<'a> {
        FdtIter::new(self.clone())
    }
}
