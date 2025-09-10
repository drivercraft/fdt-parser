use crate::{data::Raw, FdtError, Header, ReserveEntry};

#[derive(Clone)]
pub struct Fdt<'a> {
    header: Header,
    raw: Raw<'a>,
}

impl<'a> Fdt<'a> {
    /// Create a new `Fdt` from a raw pointer and size in bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// memory region of at least `size` bytes that contains a valid device tree
    /// blob.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Fdt<'a>, FdtError> {
        let header = Header::from_ptr(ptr)?;

        let buffer = Raw::new(core::slice::from_raw_parts(ptr, header.totalsize as _));

        Ok(Fdt {
            header,
            raw: buffer,
        })
    }

    /// Get a reference to the FDT header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Get a reference to the underlying buffer.
    pub fn raw(&self) -> &'a [u8] {
        self.raw.raw()
    }

    pub fn memory_reservaion_blocks(
        &self,
    ) -> Result<impl Iterator<Item = ReserveEntry> + 'a, FdtError> {
        let mut buffer = self.raw.buffer_at(self.header.off_mem_rsvmap as usize);

        let ls = core::iter::from_fn(move || {
            let address = match buffer.take_u64() {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            let size = match buffer.take_u64() {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };

            if address == 0 && size == 0 {
                return None;
            }

            Some(Ok(ReserveEntry { address, size }))
        });

        Ok(ls.flatten())
    }
}
