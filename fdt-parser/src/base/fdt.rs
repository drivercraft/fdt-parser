use core::iter;

use super::node::*;
use crate::{
    data::{Buffer, Raw},
    FdtError, FdtRangeSilce, Header, MemoryRegion, Phandle, Property, Token,
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

    pub fn as_slice(&self) -> &'a [u8] {
        self.raw.value()
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

    pub fn all_nodes(&self) -> NodeIter<'a, 16> {
        NodeIter::new(self.clone())
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

    /// Get the reserved-memory node
    fn reserved_memory_node(&self) -> Result<Option<Node<'a>>, FdtError> {
        self.find_nodes("/reserved-memory").next().transpose()
    }

    /// Get all reserved-memory child nodes (memory regions)
    pub fn reserved_memory_regions(&self) -> Result<ReservedMemoryRegionsIter<'a>, FdtError> {
        match self.reserved_memory_node()? {
            Some(reserved_memory_node) => Ok(ReservedMemoryRegionsIter::new(reserved_memory_node)),
            None => Ok(ReservedMemoryRegionsIter::empty()),
        }
    }
}

/// Iterator for reserved memory regions (child nodes of reserved-memory)
pub struct ReservedMemoryRegionsIter<'a> {
    child_iter: Option<NodeChildIter<'a>>,
}

impl<'a> ReservedMemoryRegionsIter<'a> {
    /// Create a new iterator for reserved memory regions
    fn new(reserved_memory_node: Node<'a>) -> Self {
        ReservedMemoryRegionsIter {
            child_iter: Some(reserved_memory_node.children()),
        }
    }

    /// Create an empty iterator
    fn empty() -> Self {
        ReservedMemoryRegionsIter { child_iter: None }
    }

    /// Find a reserved memory region by name
    pub fn find_by_name(self, name: &str) -> Result<Option<Node<'a>>, FdtError> {
        for region_result in self {
            let region = region_result?;
            if region.name() == name {
                return Ok(Some(region));
            }
        }
        Ok(None)
    }

    /// Find reserved memory regions by compatible string
    pub fn find_by_compatible(
        self,
        compatible: &str,
    ) -> Result<alloc::vec::Vec<Node<'a>>, FdtError> {
        let mut matching_regions = alloc::vec::Vec::new();

        for region_result in self {
            let region = region_result?;
            if let Some(compatibles) = region.compatibles()? {
                for comp in compatibles {
                    if comp == compatible {
                        matching_regions.push(region);
                        break;
                    }
                }
            }
        }

        Ok(matching_regions)
    }
}

impl<'a> Iterator for ReservedMemoryRegionsIter<'a> {
    type Item = Result<Node<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.child_iter {
            Some(iter) => iter.next(),
            None => None,
        }
    }
}

/// Stack frame for tracking node context during iteration
#[derive(Clone)]
struct NodeStackFrame<'a> {
    level: usize,
    node: NodeBase<'a>,
    address_cells: u8,
    size_cells: u8,
    ranges: Option<FdtRangeSilce<'a>>,
    interrupt_parent: Option<Phandle>,
}

pub struct NodeIter<'a, const MAX_DEPTH: usize = 16> {
    buffer: Buffer<'a>,
    fdt: Fdt<'a>,
    level: isize,
    has_err: bool,
    // Stack to store complete node hierarchy
    node_stack: heapless::Vec<NodeStackFrame<'a>, MAX_DEPTH>,
}

impl<'a, const MAX_DEPTH: usize> NodeIter<'a, MAX_DEPTH> {
    /// Create a new NodeIter with the given FDT
    pub fn new(fdt: Fdt<'a>) -> Self {
        NodeIter {
            buffer: fdt.raw.begin_at(fdt.header.off_dt_struct as usize).buffer(),
            fdt,
            level: -1,
            has_err: false,
            node_stack: heapless::Vec::new(),
        }
    }

    /// Get the current node from stack (parent of the node being created)
    fn current_parent(&self) -> Option<&NodeBase<'a>> {
        self.node_stack.last().map(|frame| &frame.node)
    }

    /// Get the current effective interrupt parent phandle from the stack
    fn current_interrupt_parent(&self) -> Option<Phandle> {
        // Search from the top of the stack downward for the first interrupt parent
        for frame in self.node_stack.iter().rev() {
            if let Some(phandle) = frame.interrupt_parent {
                return Some(phandle);
            }
        }
        None
    }

    /// Get address_cells and size_cells from parent frame
    fn current_cells(&self) -> (u8, u8) {
        self.node_stack
            .last()
            .map(|frame| (frame.address_cells, frame.size_cells))
            .unwrap_or((2, 1))
    }

    /// Push a new node onto the stack
    fn push_node(&mut self, frame: NodeStackFrame<'a>) -> Result<(), FdtError> {
        self.node_stack
            .push(frame)
            .map_err(|_| FdtError::BufferTooSmall {
                pos: self.node_stack.len(),
            })
    }

    /// Pop nodes from stack when exiting to a certain level
    fn pop_to_level(&mut self, target_level: isize) {
        while let Some(frame) = self.node_stack.last() {
            if frame.level as isize > target_level {
                self.node_stack.pop();
            } else {
                break;
            }
        }
    }

    /// Scan ahead to find node properties (#address-cells, #size-cells, interrupt-parent, ranges)
    fn scan_node_properties(
        &self,
    ) -> Result<
        (
            Option<u8>,
            Option<u8>,
            Option<Phandle>,
            Option<Property<'a>>,
        ),
        FdtError,
    > {
        let mut address_cells = None;
        let mut size_cells = None;
        let mut interrupt_parent = self.current_interrupt_parent();
        let mut ranges = None;
        let mut temp_buffer = self.buffer.clone();

        // Look for properties in this node
        loop {
            match temp_buffer.take_token() {
                Ok(Token::Prop) => {
                    let prop = temp_buffer.take_prop(&self.fdt)?;
                    match prop.name {
                        "#address-cells" => {
                            if let Ok(value) = prop.u32() {
                                address_cells = Some(value as u8);
                            }
                        }
                        "#size-cells" => {
                            if let Ok(value) = prop.u32() {
                                size_cells = Some(value as u8);
                            }
                        }
                        "interrupt-parent" => {
                            if let Ok(phandle_value) = prop.u32() {
                                interrupt_parent = Some(Phandle::from(phandle_value));
                            }
                        }
                        "ranges" => {
                            ranges = Some(prop);
                        }
                        _ => {}
                    }
                }
                Ok(Token::BeginNode) | Ok(Token::EndNode) | Ok(Token::End) => {
                    break;
                }
                _ => {
                    continue;
                }
            }
        }

        Ok((address_cells, size_cells, interrupt_parent, ranges))
    }

    /// Handle BeginNode token and create a new node
    fn handle_begin_node(&mut self) -> Result<Option<NodeBase<'a>>, FdtError> {
        self.level += 1;

        let name = self.buffer.take_str()?;
        self.buffer.take_to_aligned();

        // Scan node properties including ranges
        let (address_cells, size_cells, interrupt_parent, ranges_prop) =
            self.scan_node_properties()?;

        // Use defaults from parent if not specified
        let (default_addr, default_size) = self.current_cells();
        let address_cells = address_cells.unwrap_or(default_addr);
        let size_cells = size_cells.unwrap_or(default_size);
        let interrupt_parent = interrupt_parent.or_else(|| self.current_interrupt_parent());

        // Get parent node and its info from stack
        let parent = self.current_parent();
        let (parent_address_cells, parent_size_cells, parent_ranges) = self
            .node_stack
            .last()
            .map(|frame| {
                (
                    Some(frame.address_cells),
                    Some(frame.size_cells),
                    frame.ranges.clone(),
                )
            })
            .unwrap_or((None, None, None));

        // Calculate ranges for this node if ranges property exists
        // The ranges will be used by this node's children for address translation
        let ranges = if let Some(ranges_prop) = ranges_prop {
            // Get parent's address cells for the ranges property
            let parent_addr_cells = parent_address_cells.unwrap_or(2);

            Some(FdtRangeSilce::new(
                address_cells,
                parent_addr_cells,
                size_cells,
                &ranges_prop.data,
            ))
        } else {
            None
        };

        // Create the new node with parent info from stack
        let node = NodeBase::new_with_parent_info(
            name,
            self.fdt.clone(),
            self.buffer.remain(),
            self.level as _,
            parent,
            parent_address_cells,
            parent_size_cells,
            parent_ranges,
            interrupt_parent,
        );

        // Push this node onto the stack for its children
        let frame = NodeStackFrame {
            level: self.level as usize,
            node: node.clone(),
            address_cells,
            size_cells,
            ranges,
            interrupt_parent,
        };
        self.push_node(frame)?;

        // Return the node immediately
        Ok(Some(node))
    }

    /// Handle EndNode token - just pop from stack
    fn handle_end_node(&mut self) -> Option<NodeBase<'a>> {
        self.level -= 1;

        // Pop the current level from stack
        self.pop_to_level(self.level);

        // Don't return anything - nodes are returned on BeginNode
        None
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

impl<'a, const MAX_DEPTH: usize> Iterator for NodeIter<'a, MAX_DEPTH> {
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

struct IterFindNode<'a, const MAX_DEPTH: usize = 16> {
    itr: NodeIter<'a, MAX_DEPTH>,
    want: &'a str,
    want_itr: usize,
    is_path_last: bool,
    has_err: bool,
}

impl<'a, const MAX_DEPTH: usize> IterFindNode<'a, MAX_DEPTH> {
    fn new(itr: NodeIter<'a, MAX_DEPTH>, want: &'a str) -> Self {
        IterFindNode {
            itr,
            want,
            want_itr: 0,
            is_path_last: false,
            has_err: false,
        }
    }
}

impl<'a, const MAX_DEPTH: usize> Iterator for IterFindNode<'a, MAX_DEPTH> {
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
