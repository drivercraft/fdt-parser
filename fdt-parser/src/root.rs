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

    pub fn memory_reservaion_blocks(&self) -> impl Iterator<Item = ReserveEntry> + 'a {
        let mut buffer = self.raw.buffer_at(self.header.off_mem_rsvmap as usize);

        core::iter::from_fn(move || {
            let address = buffer.take_u64().ok()?;
            let size = buffer.take_u64().ok()?;

            if address == 0 && size == 0 {
                return None;
            }

            Some(ReserveEntry { address, size })
        })
    }
}
