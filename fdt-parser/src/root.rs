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

    /// This field shall contain the physical ID of the system’s boot CPU. It shall be identical to the physical ID given in the
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

    fn get_str(&self, offset: usize) -> Result<&'a str, FdtError> {
        let start = self.header.off_dt_strings as usize + offset;
        let mut buffer = self.raw.buffer_at(start);
        buffer.take_str()
    }

    pub fn all_nodes(&self) -> NodeIterator<'a> {
        NodeIterator {
            fdt: self.clone(),
            buffer: self.raw.buffer_at(self.header.off_dt_struct as usize),
            current_level: 0,
            finished: false,
        }
    }
}

pub struct NodeIterator<'a> {
    fdt: Fdt<'a>,
    buffer: Buffer<'a>,
    current_level: usize,
    finished: bool,
}

impl<'a> Iterator for NodeIterator<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            let token = match self.buffer.take_token() {
                Ok(token) => token,
                Err(_) => {
                    self.finished = true;
                    return None;
                }
            };

            match token {
                Token::BeginNode => {
                    // Read the node name (null-terminated string)
                    let node_name = match self.buffer.take_str() {
                        Ok(name) => name,
                        Err(_) => {
                            self.finished = true;
                            return None;
                        }
                    };

                    let node = Node::new(&self.fdt, node_name, self.current_level, 0);
                    self.current_level += 1;

                    return Some(node);
                }
                Token::EndNode => {
                    if self.current_level > 0 {
                        self.current_level -= 1;
                    }
                    // Continue to next token
                }
                Token::Prop => {
                    // Skip property: read length, name offset, and data
                    if let Ok(len) = self.buffer.take_u32() {
                        // Skip name offset
                        if self.buffer.take_u32().is_ok() {
                            // Skip property data, with proper alignment
                            let aligned_len = (len + 3) & !3;
                            if self.buffer.take(aligned_len as usize).is_err() {
                                self.finished = true;
                                return None;
                            }
                        } else {
                            self.finished = true;
                            return None;
                        }
                    } else {
                        self.finished = true;
                        return None;
                    }
                }
                Token::Nop => {
                    // Skip NOP token
                    continue;
                }
                Token::End => {
                    self.finished = true;
                    return None;
                }
                Token::Data => {
                    // This shouldn't happen in normal FDT structure
                    self.finished = true;
                    return None;
                }
            }
        }
    }
}
