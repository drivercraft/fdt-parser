//! Node view types for safe, typed access to device tree nodes.
//!
//! `NodeView` and `NodeViewMut` provide safe handles to nodes stored in the
//! `Fdt` arena. `NodeType` and `NodeTypeMut` enums allow dispatching to
//! type-specialized views such as `MemoryNodeView` and `IntcNodeView`.

// Specialized node view modules
mod generic;
mod intc;
mod memory;

use core::fmt::Display;

use alloc::{string::String, vec::Vec};
use enum_dispatch::enum_dispatch;

use crate::{Fdt, Node, NodeId, Property, RangesEntry};

// Re-export specialized view types
pub use generic::{NodeGeneric, NodeGenericMut};
pub use intc::{IntcNodeView, IntcNodeViewMut};
pub use memory::{MemoryNodeView, MemoryNodeViewMut};

#[enum_dispatch]
pub(crate) trait ViewOp<'a> {
    fn as_view(&self) -> NodeView<'a>;
    fn display(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_view().fmt(f)
    }
}

// ---------------------------------------------------------------------------
// NodeView — immutable view
// ---------------------------------------------------------------------------

/// An immutable view of a node in the device tree.
///
/// Borrows the `Fdt` arena and a `NodeId`, providing safe read access to the
/// node and its relationships (children, parent, path).
#[derive(Clone, Copy)]
pub(crate) struct NodeView<'a> {
    fdt: *mut Fdt,
    id: NodeId,
    _marker: core::marker::PhantomData<&'a ()>, // for lifetime tracking
}

unsafe impl<'a> Send for NodeView<'a> {}

impl<'a> NodeView<'a> {
    /// Creates a new `NodeView`.
    pub(crate) fn new(fdt: &'a Fdt, id: NodeId) -> Self {
        Self {
            fdt: fdt as *const Fdt as *mut Fdt,
            id,
            _marker: core::marker::PhantomData,
        }
    }

    pub fn name(&self) -> &'a str {
        self.as_node().name()
    }

    /// Returns the underlying `NodeId`.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns a reference to the underlying `Node`.
    pub fn as_node(&self) -> &'a Node {
        self.fdt()
            .node(self.id)
            .expect("NodeView references a valid node")
    }

    pub fn as_node_mut(&mut self) -> &'a mut Node {
        self.fdt_mut()
            .node_mut(self.id)
            .expect("NodeViewMut references a valid node")
    }

    /// Returns the `Fdt` arena this view belongs to.
    pub fn fdt(&self) -> &'a Fdt {
        unsafe { &*self.fdt }
    }

    pub fn fdt_mut(&mut self) -> &'a mut Fdt {
        unsafe { &mut *self.fdt }
    }

    pub fn path(&self) -> String {
        self.fdt().path_of(self.id)
    }

    pub fn parent(&self) -> Option<NodeType<'a>> {
        self.as_node()
            .parent
            .map(|pid| NodeView::new(self.fdt(), pid).classify())
    }

    #[allow(dead_code)]
    pub fn parent_mut(&mut self) -> Option<NodeTypeMut<'a>> {
        let parent = self.as_node().parent?;
        let mut parent_view = NodeView::new(self.fdt(), parent);
        let cl = parent_view.classify_mut();
        Some(cl)
    }

    pub fn address_cells(&self) -> Option<u32> {
        self.as_node().address_cells()
    }

    pub fn size_cells(&self) -> Option<u32> {
        self.as_node().size_cells()
    }

    /// Parses the `reg` property and returns corrected register entries.
    ///
    /// Uses parent node's `ranges` property to translate bus addresses to CPU addresses.
    pub fn regs(&self) -> Vec<RegFixed> {
        let node = self.as_node();
        let reg = match node.get_property("reg") {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Get address-cells and size-cells from parent (or default 2/1)
        let (addr_cells, size_cells) = self.parent_cells();

        // Get parent's ranges for address translation
        let ranges = self.parent_ranges();

        let mut reader = reg.as_reader();
        let mut results = Vec::new();

        while let Some(child_bus_address) = reader.read_cells(addr_cells) {
            let size = if size_cells > 0 {
                reader.read_cells(size_cells)
            } else {
                None
            };

            // Convert bus address to CPU address using ranges
            let mut address = child_bus_address;
            if let Some(ref ranges) = ranges {
                for r in ranges {
                    if child_bus_address >= r.child_bus_address
                        && child_bus_address < r.child_bus_address + r.length
                    {
                        address = child_bus_address - r.child_bus_address + r.parent_bus_address;
                        break;
                    }
                }
            }

            results.push(RegFixed {
                address,
                child_bus_address,
                size,
            });
        }

        results
    }

    /// Returns (address_cells, size_cells) from the parent node (defaults: 2, 1).
    fn parent_cells(&self) -> (usize, usize) {
        if let Some(parent) = self.parent() {
            let ac = parent.as_view().address_cells().unwrap_or(2) as usize;
            let sc = parent.as_view().size_cells().unwrap_or(1) as usize;
            (ac, sc)
        } else {
            (2, 1)
        }
    }

    /// Returns the parent node's ranges entries for address translation.
    fn parent_ranges(&self) -> Option<Vec<RangesEntry>> {
        self.parent().and_then(|p| {
            let view = p.as_view();
            // Get grandparent's address-cells for parsing parent_bus_address
            let parent_addr_cells = p
                .parent()
                .and_then(|gp| gp.as_view().address_cells())
                .unwrap_or(2);
            view.as_node().ranges(parent_addr_cells)
        })
    }

    /// Sets the `reg` property from CPU addresses.
    ///
    /// Converts CPU addresses to bus addresses using parent's `ranges` property
    /// and stores them in big-endian format.
    pub fn set_regs(&mut self, regs: &[fdt_raw::RegInfo]) {
        // Get address-cells and size-cells from parent (or default 2/1)
        let (addr_cells, size_cells) = self.parent_cells();

        // Get parent's ranges for address translation
        let ranges = self.parent_ranges();

        let mut data = Vec::new();

        for reg in regs {
            // Convert CPU address to bus address
            let mut bus_address = reg.address;
            if let Some(ref ranges) = ranges {
                for r in ranges {
                    // Check if CPU address is within the range mapping
                    if reg.address >= r.parent_bus_address
                        && reg.address < r.parent_bus_address + r.length
                    {
                        // Reverse conversion: cpu_address -> bus_address
                        bus_address = reg.address - r.parent_bus_address + r.child_bus_address;
                        break;
                    }
                }
            }

            // Write bus address (big-endian)
            match addr_cells {
                1 => data.extend_from_slice(&(bus_address as u32).to_be_bytes()),
                2 => {
                    data.extend_from_slice(&((bus_address >> 32) as u32).to_be_bytes());
                    data.extend_from_slice(&((bus_address & 0xFFFF_FFFF) as u32).to_be_bytes());
                }
                n => {
                    // Handle arbitrary address cells
                    for i in 0..n {
                        let shift = (n - 1 - i) * 32;
                        data.extend_from_slice(&(((bus_address >> shift) as u32).to_be_bytes()));
                    }
                }
            }

            // Write size (big-endian)
            let size = reg.size.unwrap_or(0);
            match size_cells {
                1 => data.extend_from_slice(&(size as u32).to_be_bytes()),
                2 => {
                    data.extend_from_slice(&((size >> 32) as u32).to_be_bytes());
                    data.extend_from_slice(&((size & 0xFFFF_FFFF) as u32).to_be_bytes());
                }
                n => {
                    for i in 0..n {
                        let shift = (n - 1 - i) * 32;
                        data.extend_from_slice(&(((size >> shift) as u32).to_be_bytes()));
                    }
                }
            }
        }

        let prop = Property::new("reg", data);
        self.as_node_mut().set_property(prop);
    }

    pub(crate) fn classify(&self) -> NodeType<'a> {
        if let Some(node) = MemoryNodeView::try_from_view(*self) {
            return NodeType::Memory(node);
        }

        if let Some(node) = IntcNodeView::try_from_view(*self) {
            return NodeType::InterruptController(node);
        }

        NodeType::Generic(NodeGeneric { inner: *self })
    }

    pub(crate) fn classify_mut(&mut self) -> NodeTypeMut<'a> {
        if let Some(node) = MemoryNodeViewMut::try_from_view(*self) {
            return NodeTypeMut::Memory(node);
        }

        if let Some(node) = IntcNodeViewMut::try_from_view(*self) {
            return NodeTypeMut::InterruptController(node);
        }

        NodeTypeMut::Generic(NodeGenericMut { inner: *self })
    }
}

impl core::fmt::Display for NodeView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.path())?;
        for prop in self.as_node().properties() {
            write!(f, "\n  {} = ", prop.name())?;
            if prop.name() == "compatible" {
                write!(f, "[")?;
                let strs: Vec<&str> = prop.as_str_iter().collect();
                for (i, s) in strs.iter().enumerate() {
                    write!(f, "\"{}\"", s)?;
                    if i < strs.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")?;
                continue;
            }
            if let Some(s) = prop.as_str() {
                write!(f, "\"{}\";", s)?;
            } else {
                for cell in prop.get_u32_iter() {
                    write!(f, "{:#x} ", cell)?;
                }
                write!(f, ";")?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// NodeType — classified immutable view enum
// ---------------------------------------------------------------------------

#[enum_dispatch(ViewOp)]
/// Typed node view enum, allowing pattern matching by node kind.
pub enum NodeType<'a> {
    /// A memory node (`device_type = "memory"` or name starts with "memory").
    Memory(MemoryNodeView<'a>),
    /// An interrupt controller node (has the `interrupt-controller` property).
    InterruptController(IntcNodeView<'a>),
    /// A generic node (no special classification).
    Generic(NodeGeneric<'a>),
}

impl<'a> NodeType<'a> {
    /// Returns the underlying `Node` reference.
    pub fn as_node(&self) -> &'a Node {
        self.as_view().as_node()
    }

    /// Returns the node's full path string.
    pub fn path(&self) -> String {
        self.as_view().path()
    }

    pub fn parent(&self) -> Option<NodeType<'a>> {
        self.as_view().parent()
    }

    /// Returns the node's ID.
    pub fn id(&self) -> NodeId {
        self.as_view().id()
    }

    /// Returns the node's name.
    pub fn name(&self) -> &'a str {
        self.as_view().name()
    }

    /// Parses the `reg` property and returns corrected register entries.
    pub fn regs(&self) -> Vec<RegFixed> {
        self.as_view().regs()
    }
}

impl core::fmt::Display for NodeType<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.display(f)
    }
}

// ---------------------------------------------------------------------------
// NodeTypeMut — classified mutable view enum
// ---------------------------------------------------------------------------

/// Typed mutable node view enum.
#[enum_dispatch(ViewOp)]
pub enum NodeTypeMut<'a> {
    Memory(MemoryNodeViewMut<'a>),
    InterruptController(IntcNodeViewMut<'a>),
    Generic(NodeGenericMut<'a>),
}

impl<'a> NodeTypeMut<'a> {
    /// Returns the inner node ID regardless of variant.
    pub fn id(&self) -> NodeId {
        self.as_view().id()
    }

    /// Sets the `reg` property from CPU addresses.
    ///
    /// Converts CPU addresses to bus addresses using parent's `ranges` property
    /// and stores them in big-endian format.
    pub fn set_regs(&mut self, regs: &[fdt_raw::RegInfo]) {
        self.as_view().set_regs(regs);
    }
}

// ---------------------------------------------------------------------------
// Fdt convenience methods returning views
// ---------------------------------------------------------------------------

impl Fdt {
    /// Returns a `NodeView` for the given node ID, if it exists.
    fn view(&self, id: NodeId) -> Option<NodeView<'_>> {
        if self.node(id).is_some() {
            Some(NodeView::new(self, id))
        } else {
            None
        }
    }

    /// Returns a classified `NodeType` for the given node ID.
    pub fn view_typed(&self, id: NodeId) -> Option<NodeType<'_>> {
        self.view(id).map(|v| v.classify())
    }

    /// Returns a classified `NodeTypeMut` for the given node ID.
    pub fn view_typed_mut(&mut self, id: NodeId) -> Option<NodeTypeMut<'_>> {
        self.view(id).map(|mut v| v.classify_mut())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RegFixed {
    pub address: u64,
    pub child_bus_address: u64,
    pub size: Option<u64>,
}
