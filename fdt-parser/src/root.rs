use crate::{
    data::{Buffer, Raw},
    node::Node,
    FdtError, Header, ReserveEntry, Token,
};

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
        let header = unsafe { Header::from_ptr(ptr)? };

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
        self.raw.value()
    }

    /// Get the FDT version
    pub fn version(&self) -> u32 {
        self.header.version
    }

    pub fn memory_reservaion_blocks(&self) -> impl Iterator<Item = ReserveEntry> + 'a {
        let mut buffer = self
            .raw
            .begin_at(self.header.off_mem_rsvmap as usize)
            .buffer();

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
        let mut buffer = self.raw.begin_at(start).buffer();
        buffer.take_str()
    }

    pub fn all_nodes(&self) -> NodeIter<'a> {
        NodeIter {
            buffer: self
                .raw
                .begin_at(self.header.off_dt_struct as usize)
                .buffer(),
            fdt: self.clone(),
            level: -1,
            parent: None,
            node: None,
        }
    }
}

pub struct NodeIter<'a> {
    buffer: Buffer<'a>,
    fdt: Fdt<'a>,
    level: isize,
    parent: Option<Node<'a>>,
    node: Option<Node<'a>>,
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let token = self.buffer.take_token().ok()?;
            match token {
                Token::BeginNode => {
                    self.level += 1;
                    let mut finished = None;
                    if let Some(ref p) = self.node {
                        self.parent = Some(p.clone());
                        finished = Some(p.clone());
                    }
                    let name = self.buffer.take_str().ok()?;
                    let node = Node::new(
                        name,
                        self.fdt.clone(),
                        self.buffer.remain(),
                        self.level as _,
                        self.parent.as_ref(),
                    );
                    self.node = Some(node);
                    if let Some(f) = finished {
                        return Some(f);
                    }
                }
                Token::EndNode => {
                    let node = self.node.take();
                    self.level -= 1;
                    if node.is_none() {
                        if let Some(ref p) = self.parent {
                            self.parent = p.parent().clone();
                        } else {
                            self.parent = None;
                        }
                    }
                    if let Some(n) = node {
                        return Some(n);
                    }
                }
                Token::Prop => {
                    self.buffer.take_prop(&self.fdt)?;
                }
                Token::End => {
                    return None;
                }
                _ => continue,
            }
        }
    }
}
