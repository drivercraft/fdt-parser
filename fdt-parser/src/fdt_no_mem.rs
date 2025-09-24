use core::iter;

use crate::{
    data::{Buffer, Raw},
    node::NodeBase,
    Chosen, FdtError, Header, Memory, MemoryRegion, Node, Phandle, Token,
};

#[derive(Clone)]
pub struct FdtNoMem<'a> {
    header: Header,
    pub(crate) raw: Raw<'a>,
}

impl<'a> FdtNoMem<'a> {
    /// Create a new `Fdt` from byte slice.
    pub fn from_bytes(data: &'a [u8]) -> Result<FdtNoMem<'a>, FdtError> {
        let header = Header::from_bytes(data)?;
        if data.len() < header.totalsize as usize {
            return Err(FdtError::BufferTooSmall {
                pos: header.totalsize as usize,
            });
        }
        let buffer = Raw::new(data);
        Ok(FdtNoMem {
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
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<FdtNoMem<'a>, FdtError> {
        let header = unsafe { Header::from_ptr(ptr)? };

        let raw = Raw::new(core::slice::from_raw_parts(ptr, header.totalsize as _));

        Ok(FdtNoMem { header, raw })
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

    pub fn memory_reservaion_blocks(&self) -> impl Iterator<Item = MemoryRegion> + 'a {
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

            Some(MemoryRegion {
                address: address as usize as _,
                size: size as _,
            })
        })
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
            interrupt_parent_stack: heapless::Vec::new(),
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
        let node = match node {
            Node::Chosen(c) => c,
            _ => return Err(FdtError::NodeNotFound("chosen")),
        };
        Ok(Some(node))
    }

    pub fn get_node_by_phandle(&self, phandle: Phandle) -> Result<Option<Node<'a>>, FdtError> {
        for node in self.all_nodes() {
            let node = node?;
            let phandle2 = node.phandle()?;
            if let Some(p) = phandle2 {
                if p == phandle {
                    return Ok(Some(node));
                }
            }
        }
        Ok(None)
    }

    pub fn get_node_by_name(&'a self, name: &str) -> Result<Option<Node<'a>>, FdtError> {
        for node in self.all_nodes() {
            let node = node?;
            if node.name() == name {
                return Ok(Some(node));
            }
        }
        Ok(None)
    }

    pub fn memory(&'a self) -> impl Iterator<Item = Result<Memory<'a>, FdtError>> + 'a {
        self.find_nodes("/memory@").map(|o| {
            o.map(|o| match o {
                Node::Memory(m) => m,
                _ => unreachable!(),
            })
        })
    }

    /// Reserved memory is specified as a node under the `/reserved-memory` node. The operating system shall exclude reserved
    /// memory from normal usage. One can create child nodes describing particular reserved (excluded from normal use) memory
    /// regions. Such memory regions are usually designed for the special usage by various device drivers.
    pub fn reserved_memory(&self) -> impl Iterator<Item = Result<Node<'a>, FdtError>> + 'a {
        self.find_nodes("/reserved-memory")
    }
}

pub struct NodeIter<'a> {
    buffer: Buffer<'a>,
    fdt: FdtNoMem<'a>,
    level: isize,
    parent: Option<NodeBase<'a>>,
    node: Option<NodeBase<'a>>,
    has_err: bool,
    // (level, phandle)
    interrupt_parent_stack: heapless::Vec<(usize, Phandle), 8>,
}

impl<'a> NodeIter<'a> {
    /// Get the current effective interrupt parent phandle from the stack
    fn current_interrupt_parent(&self) -> Option<Phandle> {
        self.interrupt_parent_stack
            .last()
            .map(|(_, phandle)| *phandle)
    }

    /// Push an interrupt parent to the stack for the given level
    fn push_interrupt_parent(&mut self, level: usize, phandle: Phandle) -> Result<(), FdtError> {
        self.interrupt_parent_stack
            .push((level, phandle))
            .map_err(|_| FdtError::BufferTooSmall {
                pos: self.interrupt_parent_stack.len(),
            })
    }

    /// Pop interrupt parents from the stack that are at or below the given level
    fn pop_interrupt_parents(&mut self, level: usize) {
        while let Some(&(stack_level, _)) = self.interrupt_parent_stack.last() {
            if stack_level >= level {
                self.interrupt_parent_stack.pop();
            } else {
                break;
            }
        }
    }

    /// Scan ahead to find interrupt-parent property for the current node
    fn scan_node_interrupt_parent(&self) -> Result<Option<Phandle>, FdtError> {
        let mut node_interrupt_parent = self.current_interrupt_parent();
        let mut temp_buffer = self.buffer.clone();

        // Look for interrupt-parent property in this node
        loop {
            match temp_buffer.take_token() {
                Ok(Token::Prop) => {
                    let prop = temp_buffer.take_prop(&self.fdt)?;
                    if prop.name == "interrupt-parent" {
                        if let Ok(phandle_value) = prop.u32() {
                            node_interrupt_parent = Some(Phandle::from(phandle_value));
                        }
                    }
                }
                Ok(Token::BeginNode) | Ok(Token::EndNode) | Ok(Token::End) => {
                    break;
                }
                _ => continue,
            }
        }

        Ok(node_interrupt_parent)
    }

    /// Handle BeginNode token and create a new node
    fn handle_begin_node(&mut self) -> Result<Option<NodeBase<'a>>, FdtError> {
        self.level += 1;
        let mut finished = None;
        if let Some(ref p) = self.node {
            self.parent = Some(p.clone());
            finished = Some(p.clone());
        }

        let name = self.buffer.take_str()?;
        self.buffer.take_to_aligned();

        let node_interrupt_parent = self.scan_node_interrupt_parent()?;

        let node = NodeBase::new(
            name,
            self.fdt.clone(),
            self.buffer.remain(),
            self.level as _,
            self.parent.as_ref(),
            node_interrupt_parent,
        );

        // If this node has an interrupt-parent, push it to stack for children
        if let Some(phandle) = node_interrupt_parent {
            if phandle != self.current_interrupt_parent().unwrap_or(Phandle::from(0)) {
                self.push_interrupt_parent(self.level as usize, phandle)?;
            }
        }

        self.node = Some(node);
        Ok(finished)
    }

    /// Handle EndNode token and return the completed node
    fn handle_end_node(&mut self) -> Option<NodeBase<'a>> {
        let node = self.node.take();

        // Pop interrupt parents when exiting a node
        self.pop_interrupt_parents(self.level as usize);

        self.level -= 1;
        if node.is_none() {
            if let Some(ref p) = self.parent {
                self.parent = p.parent().map(|n| n.node().clone());
            } else {
                self.parent = None;
            }
        }

        node
    }

    /// Handle Prop token
    fn handle_prop(&mut self) -> Result<(), FdtError> {
        let _prop = self.buffer.take_prop(&self.fdt)?;
        // Property handling is now done in BeginNode scanning
        Ok(())
    }

    fn try_next(&mut self) -> Result<Option<NodeBase<'a>>, FdtError> {
        loop {
            let token = self.buffer.take_token()?;
            match token {
                Token::BeginNode => {
                    if let Some(finished_node) = self.handle_begin_node()? {
                        return Ok(Some(finished_node));
                    }
                }
                Token::EndNode => {
                    if let Some(node) = self.handle_end_node() {
                        return Ok(Some(node));
                    }
                }
                Token::Prop => {
                    self.handle_prop()?;
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
            Ok(Some(node)) => Some(Ok(node.into())),
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
