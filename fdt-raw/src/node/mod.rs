//! Device tree node types and parsing.
//!
//! This module provides types for representing device tree nodes,
//! including the base node type and specialized variants like Chosen
//! and Memory nodes. It also contains the iterator logic for parsing
//! nodes from the FDT structure block.

use core::fmt;
use core::ops::Deref;
use core::{ffi::CStr, fmt::Debug};

use crate::Fdt;
use crate::fmt_utils;
use crate::{
    FdtError, Token,
    data::{Bytes, Reader, U32_SIZE},
};

mod chosen;
mod memory;
mod prop;

pub use chosen::Chosen;
pub use memory::{Memory, MemoryRegion};
pub use prop::{PropIter, Property, RangeInfo, RegInfo, RegIter, VecRange};

/// Context inherited from a node's parent.
///
/// Contains the `#address-cells` and `#size-cells` values that should
/// be used when parsing properties of the current node. These values
/// are inherited from the parent node unless overridden.
///
/// # Default Values
///
/// The root node defaults to `#address-cells = 2` and `#size-cells = 1`
/// per the Device Tree specification.
#[derive(Clone)]
pub(crate) struct NodeContext {
    /// Parent node's #address-cells (used for parsing current node's reg)
    pub address_cells: u8,
    /// Parent node's #size-cells (used for parsing current node's reg)
    pub size_cells: u8,
}

impl Default for NodeContext {
    fn default() -> Self {
        NodeContext {
            address_cells: 2,
            size_cells: 1,
        }
    }
}

/// Base device tree node structure.
///
/// Contains the common data and methods available on all nodes,
/// including name, level, properties, and cell values.
#[derive(Clone)]
pub struct NodeBase<'a> {
    name: &'a str,
    data: Bytes<'a>,
    strings: Bytes<'a>,
    level: usize,
    _fdt: Fdt<'a>,
    /// Current node's #address-cells (used for child nodes)
    pub address_cells: u8,
    /// Current node's #size-cells (used for child nodes)
    pub size_cells: u8,
    /// Inherited context (contains parent's cells)
    context: NodeContext,
    /// Path components from root to this node
    path_components: heapless::Vec<&'a str, 16>,
}

impl<'a> NodeBase<'a> {
    /// Returns the node's name.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the depth/level of this node in the tree.
    pub fn level(&self) -> usize {
        self.level
    }

    /// Returns an iterator over this node's properties.
    pub fn properties(&self) -> PropIter<'a> {
        PropIter::new(self.data.reader(), self.strings.clone())
    }

    /// Finds a property by name.
    pub fn find_property(&self, name: &str) -> Option<Property<'a>> {
        self.properties().find(|p| p.name() == name)
    }

    /// Finds a string property by name.
    pub fn find_property_str(&self, name: &str) -> Option<&'a str> {
        let prop = self.find_property(name)?;
        prop.as_str()
    }

    /// Finds and parses the `reg` property, returning a Reg iterator.
    pub fn reg(&self) -> Option<RegIter<'a>> {
        let prop = self.find_property("reg")?;
        Some(RegIter::new(
            prop.data().reader(),
            self.context.address_cells,
            self.context.size_cells,
        ))
    }

    /// Finds and parses the `reg` property, returning all RegInfo entries.
    pub fn reg_array<const N: usize>(&self) -> heapless::Vec<RegInfo, N> {
        let mut result = heapless::Vec::new();
        if let Some(reg) = self.reg() {
            for info in reg {
                if result.push(info).is_err() {
                    break; // Array is full
                }
            }
        }
        result
    }

    /// Checks if this is the chosen node.
    fn is_chosen(&self) -> bool {
        self.name == "chosen"
    }

    /// Checks if this is a memory node.
    fn is_memory(&self) -> bool {
        self.name.starts_with("memory")
    }

    /// Returns the `ranges` property if present.
    pub fn ranges(&self) -> Option<VecRange<'a>> {
        let prop = self.find_property("ranges")?;
        Some(VecRange::new(
            self.address_cells as usize,
            self.context.address_cells as usize,
            self.context.size_cells as usize,
            prop.data(),
        ))
    }

    /// Returns an iterator over compatible strings.
    pub fn compatibles(&self) -> impl Iterator<Item = &'a str> {
        self.find_property("compatible")
            .into_iter()
            .flat_map(|p| p.as_str_iter())
    }

    /// Returns the full path of this node as a string.
    ///
    /// For the root node, returns "/". For other nodes, returns the
    /// absolute path like "/soc/serial@0".
    pub fn path(&self) -> heapless::String<256> {
        let mut result = heapless::String::new();
        if self.path_components.is_empty() {
            let _ = result.push('/');
            return result;
        }
        for component in &self.path_components {
            let _ = result.push('/');
            let _ = result.push_str(component);
        }
        result
    }
}

impl fmt::Display for NodeBase<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_utils::write_indent(f, self.level, "    ")?;
        let name = if self.name.is_empty() { "/" } else { self.name };

        writeln!(f, "{} {{", name)?;
        for prop in self.properties() {
            fmt_utils::write_indent(f, self.level + 1, "    ")?;
            writeln!(f, "{};", prop)?;
        }
        fmt_utils::write_indent(f, self.level, "    ")?;
        write!(f, "}}")
    }
}

// ============================================================================
// Node enum: supports specialized node types
// ============================================================================

/// Device tree node enum supporting specialized node types.
///
/// Nodes are automatically classified into General, Chosen, or Memory
/// variants based on their name and properties.
#[derive(Clone)]
pub enum Node<'a> {
    /// A general-purpose node without special handling
    General(NodeBase<'a>),
    /// The /chosen node containing boot parameters
    Chosen(Chosen<'a>),
    /// A memory node describing physical memory layout
    Memory(Memory<'a>),
}

impl<'a> From<NodeBase<'a>> for Node<'a> {
    fn from(node: NodeBase<'a>) -> Self {
        if node.is_chosen() {
            Node::Chosen(Chosen::new(node))
        } else if node.is_memory() {
            Node::Memory(Memory::new(node))
        } else {
            Node::General(node)
        }
    }
}

impl<'a> Deref for Node<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            Node::General(n) => n,
            Node::Chosen(c) => c.deref(),
            Node::Memory(m) => m.deref(),
        }
    }
}

impl fmt::Display for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::General(n) => f.debug_tuple("General").field(&n.name()).finish(),
            Node::Chosen(c) => c.fmt(f),
            Node::Memory(m) => m.fmt(f),
        }
    }
}

/// Key information extracted when parsing properties.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedProps {
    pub address_cells: Option<u8>,
    pub size_cells: Option<u8>,
}

/// State of a single node iteration.
///
/// Tracks the current state while parsing a single node's content.
/// Used internally by `OneNodeIter` to communicate with `FdtIter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OneNodeState {
    /// Currently processing the node (reading properties)
    Processing,
    /// Encountered a child's BeginNode token, needs to backtrack
    ChildBegin,
    /// Encountered EndNode token, current node processing complete
    End,
}

/// An iterator over a single node's content.
///
/// When encountering a child's BeginNode token, it backtracks and signals
/// FdtIter to handle the child node. This allows FdtIter to maintain
/// proper tree traversal state.
///
/// # Implementation Notes
///
/// This iterator is `pub(crate)` because it's an internal implementation
/// detail of the FDT parsing machinery. External consumers should use
/// `FdtIter` or `NodeBase::properties()` instead.
pub(crate) struct OneNodeIter<'a> {
    /// Reader for the node's property data
    reader: Reader<'a>,
    /// Strings block for looking up property names
    strings: Bytes<'a>,
    /// Current iteration state
    state: OneNodeState,
    /// Depth level of this node in the tree
    level: usize,
    /// Inherited context from parent (address_cells, size_cells)
    context: NodeContext,
    /// Extracted properties (#address-cells, #size-cells)
    parsed_props: ParsedProps,
    /// Reference to the containing FDT for path resolution
    fdt: Fdt<'a>,
}

impl<'a> OneNodeIter<'a> {
    /// Creates a new single node iterator.
    pub fn new(
        reader: Reader<'a>,
        strings: Bytes<'a>,
        level: usize,
        context: NodeContext,
        fdt: Fdt<'a>,
    ) -> Self {
        Self {
            reader,
            strings,
            state: OneNodeState::Processing,
            level,
            context,
            parsed_props: ParsedProps::default(),
            fdt,
        }
    }

    /// Returns a reference to the reader.
    pub fn reader(&self) -> &Reader<'a> {
        &self.reader
    }

    /// Returns the parsed properties extracted from this node.
    pub fn parsed_props(&self) -> &ParsedProps {
        &self.parsed_props
    }

    /// Reads the node name (called after BeginNode token).
    ///
    /// Reads the null-terminated node name and aligns to a 4-byte boundary.
    /// Returns a partially-constructed `NodeBase` with default cell values
    /// that will be updated by `process()`.
    pub fn read_node_name(
        &mut self,
        parent_path: &heapless::Vec<&'a str, 16>,
    ) -> Result<NodeBase<'a>, FdtError> {
        // Read null-terminated name string
        let name = self.read_cstr()?;

        // Align to 4-byte boundary
        self.align4();

        let data = self.reader.remain();

        // Build path components: parent path + current node name
        let mut path_components = parent_path.clone();
        if !name.is_empty() {
            let _ = path_components.push(name);
        }

        Ok(NodeBase {
            name,
            data,
            strings: self.strings.clone(),
            level: self.level,
            // Default values, will be updated in process()
            address_cells: 2,
            size_cells: 1,
            context: self.context.clone(),
            _fdt: self.fdt.clone(),
            path_components,
        })
    }

    /// Reads a null-terminated string from the current position.
    fn read_cstr(&mut self) -> Result<&'a str, FdtError> {
        let bytes = self.reader.remain();
        let cstr = CStr::from_bytes_until_nul(bytes.as_slice())?;
        let s = cstr.to_str()?;
        // Skip string content + null terminator
        let _ = self.reader.read_bytes(s.len() + 1);
        Ok(s)
    }

    /// Aligns the reader to a 4-byte boundary.
    ///
    /// FDT structures are 4-byte aligned, so after reading variable-length
    /// data (like node names), we need to pad to the next 4-byte boundary.
    fn align4(&mut self) {
        let pos = self.reader.position();
        let aligned = (pos + U32_SIZE - 1) & !(U32_SIZE - 1);
        let skip = aligned - pos;
        if skip > 0 {
            let _ = self.reader.read_bytes(skip);
        }
    }

    /// Reads a property name from the strings block.
    ///
    /// Property names are stored as offsets into the strings block,
    /// not inline with the property data.
    fn read_prop_name(&self, nameoff: u32) -> Result<&'a str, FdtError> {
        let bytes = self.strings.slice(nameoff as usize..self.strings.len());
        let cstr = CStr::from_bytes_until_nul(bytes.as_slice())?;
        Ok(cstr.to_str()?)
    }

    /// Reads a u32 value from big-endian bytes at the given offset.
    fn read_u32_be(data: &[u8], offset: usize) -> u64 {
        u32::from_be_bytes(data[offset..offset + U32_SIZE].try_into().unwrap()) as u64
    }

    /// Processes node content, parsing properties until child node or end.
    ///
    /// This is the core parsing loop for a node. It reads tokens sequentially:
    /// - Properties are parsed and `#address-cells`/`#size-cells` are extracted
    /// - Child nodes cause backtracking and return `ChildBegin`
    /// - EndNode terminates processing and returns `End`
    ///
    /// # Returns
    ///
    /// - `Ok(OneNodeState::ChildBegin)` if a child node was found
    /// - `Ok(OneNodeState::End)` if the node ended
    /// - `Err(FdtError)` if parsing failed
    pub fn process(&mut self) -> Result<OneNodeState, FdtError> {
        loop {
            let token = self.reader.read_token()?;
            match token {
                Token::BeginNode => {
                    // Child node encountered, backtrack token and return
                    self.reader.backtrack(U32_SIZE);
                    self.state = OneNodeState::ChildBegin;
                    return Ok(OneNodeState::ChildBegin);
                }
                Token::EndNode => {
                    self.state = OneNodeState::End;
                    return Ok(OneNodeState::End);
                }
                Token::Prop => {
                    // Read property: len and nameoff
                    let len = self.reader.read_u32().ok_or(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    })? as usize;

                    let nameoff = self.reader.read_u32().ok_or(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    })?;

                    // Read property data
                    let prop_data = if len > 0 {
                        self.reader
                            .read_bytes(len)
                            .ok_or(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            })?
                    } else {
                        Bytes::new(&[])
                    };

                    // Parse key properties
                    if let Ok(prop_name) = self.read_prop_name(nameoff) {
                        match prop_name {
                            "#address-cells" if len == 4 => {
                                self.parsed_props.address_cells =
                                    Some(Self::read_u32_be(&prop_data, 0) as u8);
                            }
                            "#size-cells" if len == 4 => {
                                self.parsed_props.size_cells =
                                    Some(Self::read_u32_be(&prop_data, 0) as u8);
                            }
                            _ => {}
                        }
                    }

                    // Align to 4-byte boundary
                    self.align4();
                }
                Token::Nop => {
                    // Ignore NOP tokens
                }
                Token::End => {
                    // Structure block ended
                    self.state = OneNodeState::End;
                    return Ok(OneNodeState::End);
                }
                Token::Data(_) => {
                    // Invalid token
                    return Err(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    });
                }
            }
        }
    }
}
