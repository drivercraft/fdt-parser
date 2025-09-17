use core::iter;

use crate::{
    data::{Buffer, Raw},
    node::Node,
    Chosen, FdtError, Header, ReserveEntry, Token,
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
            has_err: false,
        }
    }

    /// if path start with '/' then search by path, else search by aliases
    pub fn find_nodes(
        &self,
        path: &'a str,
    ) -> impl Iterator<Item = Result<Node<'a>, FdtError>> + 'a {
        let path = if path.starts_with("/") {
            path
        } else {
            self.find_aliase(path).unwrap()
        };

        IterFindNode::new(self.all_nodes(), path)
    }

    pub fn find_aliase(&self, name: &str) -> Result<&'a str, FdtError> {
        let aliases = self
            .find_nodes("/aliases")
            .next()
            .ok_or(FdtError::NoAlias)??;
        for prop in aliases.properties() {
            let prop = prop?;
            if prop.name.eq(name) {
                return prop.str();
            }
        }
        Err(FdtError::NoAlias)
    }

    pub fn find_compatible<'b, 'c: 'b>(
        &'b self,
        with: &'c [&'c str],
    ) -> impl Iterator<Item = Result<Node<'a>, FdtError>> + 'b {
        let mut iter = self.all_nodes();
        let mut has_err = false;
        iter::from_fn(move || loop {
            if has_err {
                return None;
            }
            let node = iter.next()?;
            let node = match node {
                Ok(n) => n,
                Err(e) => {
                    return {
                        has_err = true;
                        Some(Err(e))
                    }
                }
            };
            let comp = match node.compatibles() {
                Ok(c) => c,
                Err(e) => {
                    return {
                        has_err = true;
                        Some(Err(e))
                    }
                }
            };

            if let Some(comp) = comp {
                for c in comp {
                    if with.iter().any(|w| w.eq(&c)) {
                        return Some(Ok(node));
                    }
                }
            }
        })
    }

    pub fn chosen(&self) -> Result<Option<Chosen<'a>>, FdtError> {
        let node = none_ok!(self.find_nodes("/chosen").next())?;
        Ok(Some(Chosen::new(node)))
    }
}

pub struct NodeIter<'a> {
    buffer: Buffer<'a>,
    fdt: Fdt<'a>,
    level: isize,
    parent: Option<Node<'a>>,
    node: Option<Node<'a>>,
    has_err: bool,
}

impl<'a> NodeIter<'a> {
    fn try_next(&mut self) -> Result<Option<Node<'a>>, FdtError> {
        loop {
            let token = self.buffer.take_token()?;
            match token {
                Token::BeginNode => {
                    self.level += 1;
                    let mut finished = None;
                    if let Some(ref p) = self.node {
                        self.parent = Some(p.clone());
                        finished = Some(p.clone());
                    }
                    let name = self.buffer.take_str()?;
                    self.buffer.take_to_aligned();
                    let node = Node::new(
                        name,
                        self.fdt.clone(),
                        self.buffer.remain(),
                        self.level as _,
                        self.parent.as_ref(),
                    );
                    self.node = Some(node);
                    if let Some(f) = finished {
                        return Ok(Some(f));
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
                        return Ok(Some(n));
                    }
                }
                Token::Prop => {
                    self.buffer.take_prop(&self.fdt)?;
                }
                Token::End => {
                    return Ok(None);
                }
                _ => continue,
            }
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = Result<Node<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_err {
            return None;
        }
        match self.try_next() {
            Ok(Some(node)) => Some(Ok(node)),
            Ok(None) => None,
            Err(e) => {
                self.has_err = true;
                Some(Err(e))
            }
        }
    }
}

struct IterFindNode<'a> {
    itr: NodeIter<'a>,
    want: &'a str,
    want_itr: usize,
    is_path_last: bool,
    has_err: bool,
}

impl<'a> IterFindNode<'a> {
    fn new(itr: NodeIter<'a>, want: &'a str) -> Self {
        IterFindNode {
            itr,
            want,
            want_itr: 0,
            is_path_last: false,
            has_err: false,
        }
    }
}

impl<'a> Iterator for IterFindNode<'a> {
    type Item = Result<Node<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut out = None;
        loop {
            let mut parts = self.want.split("/").filter(|o| !o.is_empty());
            let mut want_part = "/";
            for _ in 0..self.want_itr {
                if let Some(part) = parts.next() {
                    want_part = part;
                } else {
                    self.is_path_last = true;
                    if let Some(out) = out {
                        return Some(out);
                    }
                }
            }
            let node = match self.itr.next()? {
                Ok(v) => v,
                Err(e) => {
                    self.has_err = true;
                    return Some(Err(e));
                }
            };

            let eq = if want_part.contains("@") {
                node.name().eq(want_part)
            } else {
                let name = node.name().split("@").next().unwrap();
                name.eq(want_part)
            };
            if eq {
                self.want_itr += 1;
                out = Some(Ok(node));
            }
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> Fdt<'a> {
    pub fn all_nodes_vec(&self) -> Result<alloc::vec::Vec<Node<'a>>, FdtError> {
        let mut nodes = vec![];
        for node in self.all_nodes() {
            nodes.push(node?);
        }
        Ok(nodes)
    }
}
