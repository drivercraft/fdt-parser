use crate::{data::Raw, node::Node, walk::Walker, FdtError, Header, ReserveEntry};

#[derive(Clone)]
pub struct Fdt<'a> {
    header: Header,
    pub(crate) raw: Raw<'a>,
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
        let buffer = Raw::new(data);
        Ok(Fdt {
            header,
            raw: buffer,
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
        let header = Header::from_ptr(ptr)?;

        let raw = Raw::new(core::slice::from_raw_parts(ptr, header.totalsize as _));

        Ok(Fdt { header, raw })
    }

    /// Get a reference to the FDT header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn total_size(&self) -> usize {
        self.header.totalsize as usize
    }

    /// This field shall contain the physical ID of the system's boot CPU. It shall be identical to the physical ID given in the
    /// reg property of that CPU node within the devicetree.
    pub fn boot_cpuid_phys(&self) -> u32 {
        self.header.boot_cpuid_phys
    }

    /// Get a reference to the underlying buffer.
    pub fn raw(&self) -> &'a [u8] {
        self.raw.raw()
    }

    /// Get the FDT version
    pub fn version(&self) -> u32 {
        self.header.version
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

    /// Alias for memory_reservaion_blocks for compatibility
    pub fn memory_reservation_block(&self) -> impl Iterator<Item = ReserveEntry> + 'a {
        self.memory_reservaion_blocks()
    }

    pub(crate) fn get_str(&self, offset: usize) -> Result<&'a str, FdtError> {
        let start = self.header.off_dt_strings as usize + offset;
        let mut buffer = self.raw.buffer_at(start);
        buffer.take_str()
    }

    /// 创建一个Walker实例用于各种遍历操作
    pub fn walker(&self) -> Walker<'a> {
        Walker::new(self.clone())
    }

    #[cfg(feature = "alloc")]
    pub fn all_nodes(&self) -> alloc::vec::Vec<Node<'a>> {
        let mut nodes = alloc::vec![];
        let walker = self.walker();
        let _ = walker.walk_all(|node| {
            nodes.push(node.clone());
            Ok(true)
        });
        nodes
    }
}
