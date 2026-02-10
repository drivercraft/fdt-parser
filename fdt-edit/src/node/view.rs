//! Node view types for safe, typed access to device tree nodes.
//!
//! `NodeView` and `NodeViewMut` provide safe handles to nodes stored in the
//! `Fdt` arena. `NodeType` and `NodeTypeMut` enums allow dispatching to
//! type-specialized views such as `MemoryNodeView` and `IntcNodeView`.

use core::{fmt::Display, ops::Deref};

use alloc::{string::String, vec::Vec};
use enum_dispatch::enum_dispatch;
use fdt_raw::{MemoryRegion, Phandle, Status};

use crate::{Fdt, Node, NodeId, Property, RangesEntry};

#[enum_dispatch]
pub(crate) trait ViewOp {
    fn as_view(&self) -> NodeView<'_>;
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
pub struct NodeView<'a> {
    fdt: &'a Fdt,
    id: NodeId,
}

impl<'a> NodeView<'a> {
    /// Creates a new `NodeView`.
    pub(crate) fn new(fdt: &'a Fdt, id: NodeId) -> Self {
        Self { fdt, id }
    }

    /// Returns the underlying `NodeId`.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns a reference to the underlying `Node`.
    pub fn as_node(&self) -> &'a Node {
        self.fdt
            .node(self.id)
            .expect("NodeView references a valid node")
    }

    /// Returns the `Fdt` arena this view belongs to.
    pub fn fdt(&self) -> &'a Fdt {
        self.fdt
    }

    // -- delegation to Node --

    /// Node name (without path).
    pub fn name(&self) -> &'a str {
        self.as_node().name()
    }

    /// Property list.
    pub fn properties(&self) -> &'a [Property] {
        self.as_node().properties()
    }

    /// Get a property by name.
    pub fn get_property(&self, name: &str) -> Option<&'a Property> {
        self.as_node().get_property(name)
    }

    /// Child node IDs.
    pub fn child_ids(&self) -> &'a [NodeId] {
        self.as_node().children()
    }

    /// Iterator over child `NodeView`s.
    pub fn children(&self) -> impl Iterator<Item = NodeView<'a>> + 'a {
        let fdt = self.fdt;
        self.as_node()
            .children()
            .iter()
            .map(move |&child_id| NodeView::new(fdt, child_id))
    }

    /// Get a child view by name.
    pub fn get_child(&self, name: &str) -> Option<NodeView<'a>> {
        let child_id = self.as_node().get_child(name)?;
        Some(NodeView::new(self.fdt, child_id))
    }

    /// Parent view, if any.
    pub fn parent(&self) -> Option<NodeView<'a>> {
        self.as_node()
            .parent
            .map(|pid| NodeView::new(self.fdt, pid))
    }

    /// Full path string (e.g. "/soc/uart@10000").
    pub fn path(&self) -> String {
        self.fdt.path_of(self.id)
    }

    // -- DT shortcut methods (delegated to Node) --

    pub fn address_cells(&self) -> Option<u32> {
        self.as_node().address_cells()
    }
    pub fn size_cells(&self) -> Option<u32> {
        self.as_node().size_cells()
    }
    pub fn phandle(&self) -> Option<Phandle> {
        self.as_node().phandle()
    }
    pub fn interrupt_parent(&self) -> Option<Phandle> {
        self.as_node().interrupt_parent()
    }
    pub fn status(&self) -> Option<Status> {
        self.as_node().status()
    }
    pub fn compatible(&self) -> Option<impl Iterator<Item = &'a str>> {
        self.as_node().compatible()
    }
    pub fn compatibles(&self) -> impl Iterator<Item = &'a str> {
        self.as_node().compatibles()
    }
    pub fn device_type(&self) -> Option<&'a str> {
        self.as_node().device_type()
    }
    pub fn ranges(&self, parent_address_cells: u32) -> Option<Vec<RangesEntry>> {
        self.as_node().ranges(parent_address_cells)
    }

    // -- classification --

    /// Classify this node into a typed view.
    pub fn classify(&self) -> NodeType<'a> {
        let node = self.as_node();
        if node.is_interrupt_controller() {
            NodeType::InterruptController(IntcNodeView { inner: *self })
        } else if node.is_memory() {
            NodeType::Memory(MemoryNodeView { inner: *self })
        } else {
            NodeType::Generic(GenericNodeView { inner: *self })
        }
    }
}

impl core::fmt::Display for NodeView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.path())?;
        for prop in self.properties() {
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
// NodeViewMut — mutable view
// ---------------------------------------------------------------------------

/// A mutable view of a node in the device tree.
///
/// Holds an exclusive `&mut Fdt` reference and a `NodeId`.
pub struct NodeViewMut<'a> {
    fdt: &'a mut Fdt,
    id: NodeId,
}

impl<'a> NodeViewMut<'a> {
    /// Creates a new `NodeViewMut`.
    pub(crate) fn new(fdt: &'a mut Fdt, id: NodeId) -> Self {
        Self { fdt, id }
    }

    /// Returns the underlying `NodeId`.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Immutable access to the underlying `Node`.
    pub fn as_node(&self) -> &Node {
        self.fdt
            .node(self.id)
            .expect("NodeViewMut references a valid node")
    }

    /// Mutable access to the underlying `Node`.
    pub fn as_node_mut(&mut self) -> &mut Node {
        self.fdt
            .node_mut(self.id)
            .expect("NodeViewMut references a valid node")
    }

    /// Node name.
    pub fn name(&self) -> &str {
        self.as_node().name()
    }

    /// Full path string.
    pub fn path(&self) -> String {
        self.fdt.path_of(self.id)
    }

    // -- property mutation --

    /// Set a property (add or update).
    pub fn set_property(&mut self, prop: Property) {
        self.as_node_mut().set_property(prop);
    }

    /// Remove a property by name.
    pub fn remove_property(&mut self, name: &str) -> Option<Property> {
        self.as_node_mut().remove_property(name)
    }

    /// Get a property by name.
    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.as_node().get_property(name)
    }

    // -- child mutation --

    /// Add a child node, returning the new child's ID.
    pub fn add_child(&mut self, node: Node) -> NodeId {
        self.fdt.add_node(self.id, node)
    }

    /// Remove a child node by name.
    pub fn remove_child(&mut self, name: &str) -> Option<NodeId> {
        self.fdt.remove_node(self.id, name)
    }

    // -- classification (consumes self because &mut Fdt cannot be copied) --

    /// Classify this node into a typed mutable view.
    pub fn classify(self) -> NodeTypeMut<'a> {
        let is_intc = self
            .fdt
            .node(self.id)
            .map(|n| n.is_interrupt_controller())
            .unwrap_or(false);
        let is_mem = self
            .fdt
            .node(self.id)
            .map(|n| n.is_memory())
            .unwrap_or(false);

        if is_intc {
            NodeTypeMut::InterruptController(IntcNodeViewMut { inner: self })
        } else if is_mem {
            NodeTypeMut::Memory(MemoryNodeViewMut { inner: self })
        } else {
            NodeTypeMut::Generic(GenericNodeViewMut { inner: self })
        }
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
    Generic(GenericNodeView<'a>),
}

impl<'a> NodeType<'a> {
    /// Returns the inner `NodeView` regardless of variant.
    pub fn as_view(&self) -> &NodeView<'a> {
        match self {
            NodeType::Memory(v) => &v.inner,
            NodeType::InterruptController(v) => &v.inner,
            NodeType::Generic(v) => &v.inner,
        }
    }

    /// Returns the underlying `Node` reference.
    pub fn as_node(&self) -> &'a Node {
        self.as_view().as_node()
    }

    /// Returns the node's full path string.
    pub fn path(&self) -> String {
        self.as_view().path()
    }

    /// Returns the node's ID.
    pub fn id(&self) -> NodeId {
        self.as_view().id()
    }

    /// Returns the node's name.
    pub fn name(&self) -> &'a str {
        self.as_view().name()
    }
}

impl core::fmt::Display for NodeType<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_view())
    }
}

// ---------------------------------------------------------------------------
// NodeTypeMut — classified mutable view enum
// ---------------------------------------------------------------------------

/// Typed mutable node view enum.
pub enum NodeTypeMut<'a> {
    Memory(MemoryNodeViewMut<'a>),
    InterruptController(IntcNodeViewMut<'a>),
    Generic(GenericNodeViewMut<'a>),
}

impl<'a> NodeTypeMut<'a> {
    /// Returns the inner node ID regardless of variant.
    pub fn id(&self) -> NodeId {
        match self {
            NodeTypeMut::Memory(v) => v.inner.id,
            NodeTypeMut::InterruptController(v) => v.inner.id,
            NodeTypeMut::Generic(v) => v.inner.id,
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryNodeView
// ---------------------------------------------------------------------------

/// Specialized view for memory nodes.
///
/// Provides methods for parsing `reg` into memory regions.
#[derive(Clone, Copy)]
pub struct MemoryNodeView<'a> {
    inner: NodeView<'a>,
}

impl ViewOp for MemoryNodeView<'_> {
    fn as_view(&self) -> NodeView<'_> {
        self.inner
    }
}

impl<'a> Deref for MemoryNodeView<'a> {
    type Target = NodeView<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> MemoryNodeView<'a> {
    /// Iterates over memory regions parsed from the `reg` property.
    ///
    /// Uses the parent node's `#address-cells` and `#size-cells` to decode.
    pub fn regions(&self) -> Vec<MemoryRegion> {
        let node = self.inner.as_node();
        let reg = match node.get_property("reg") {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Get address-cells and size-cells from parent (or default 2/1)
        let (addr_cells, size_cells) = self.parent_cells();

        let mut reader = reg.as_reader();
        let mut regions = Vec::new();

        while let (Some(address), Some(size)) =
            (reader.read_cells(addr_cells), reader.read_cells(size_cells))
        {
            regions.push(MemoryRegion { address, size });
        }

        regions
    }

    /// Total size across all memory regions.
    pub fn total_size(&self) -> u64 {
        self.regions().iter().map(|r| r.size).sum()
    }

    /// Returns (address_cells, size_cells) from the parent node (defaults: 2, 1).
    fn parent_cells(&self) -> (usize, usize) {
        if let Some(parent) = self.inner.parent() {
            let ac = parent.address_cells().unwrap_or(2) as usize;
            let sc = parent.size_cells().unwrap_or(1) as usize;
            (ac, sc)
        } else {
            (2, 1)
        }
    }
}

// ---------------------------------------------------------------------------
// IntcNodeView
// ---------------------------------------------------------------------------

/// Specialized view for interrupt controller nodes.
#[derive(Clone, Copy)]
pub struct IntcNodeView<'a> {
    inner: NodeView<'a>,
}

impl<'a> ViewOp for IntcNodeView<'a> {
    fn as_view(&self) -> NodeView<'a> {
        self.inner
    }
}

impl<'a> Deref for IntcNodeView<'a> {
    type Target = NodeView<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> IntcNodeView<'a> {
    /// Returns the `#interrupt-cells` property value.
    pub fn interrupt_cells(&self) -> Option<u32> {
        self.inner.as_node().interrupt_cells()
    }

    /// This is always `true` for `IntcNodeView` (type-level guarantee).
    pub fn is_interrupt_controller(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// GenericNodeView
// ---------------------------------------------------------------------------

/// A generic node view with no extra specialization.
#[derive(Clone, Copy)]
pub struct GenericNodeView<'a> {
    inner: NodeView<'a>,
}

impl ViewOp for GenericNodeView<'_> {
    fn as_view(&self) -> NodeView<'_> {
        self.inner
    }
}

impl<'a> Deref for GenericNodeView<'a> {
    type Target = NodeView<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// ---------------------------------------------------------------------------
// Mutable specialized views
// ---------------------------------------------------------------------------

/// Mutable view for memory nodes.
pub struct MemoryNodeViewMut<'a> {
    inner: NodeViewMut<'a>,
}

impl<'a> MemoryNodeViewMut<'a> {
    /// Access underlying mutable view.
    pub fn as_view_mut(&mut self) -> &mut NodeViewMut<'a> {
        &mut self.inner
    }

    /// Access underlying immutable node.
    pub fn as_node(&self) -> &Node {
        self.inner.as_node()
    }

    pub fn id(&self) -> NodeId {
        self.inner.id
    }
}

/// Mutable view for interrupt controller nodes.
pub struct IntcNodeViewMut<'a> {
    inner: NodeViewMut<'a>,
}

impl<'a> IntcNodeViewMut<'a> {
    pub fn as_view_mut(&mut self) -> &mut NodeViewMut<'a> {
        &mut self.inner
    }

    pub fn as_node(&self) -> &Node {
        self.inner.as_node()
    }

    pub fn id(&self) -> NodeId {
        self.inner.id
    }

    pub fn interrupt_cells(&self) -> Option<u32> {
        self.inner.as_node().interrupt_cells()
    }
}

/// Mutable view for generic nodes.
pub struct GenericNodeViewMut<'a> {
    inner: NodeViewMut<'a>,
}

impl<'a> GenericNodeViewMut<'a> {
    pub fn as_view_mut(&mut self) -> &mut NodeViewMut<'a> {
        &mut self.inner
    }

    pub fn as_node(&self) -> &Node {
        self.inner.as_node()
    }

    pub fn id(&self) -> NodeId {
        self.inner.id
    }
}

// ---------------------------------------------------------------------------
// Fdt convenience methods returning views
// ---------------------------------------------------------------------------

impl Fdt {
    /// Returns a `NodeView` for the root node.
    pub fn root(&self) -> NodeView<'_> {
        NodeView::new(self, self.root_id())
    }

    /// Returns a `NodeView` for the given node ID, if it exists.
    pub fn view(&self, id: NodeId) -> Option<NodeView<'_>> {
        if self.node(id).is_some() {
            Some(NodeView::new(self, id))
        } else {
            None
        }
    }

    /// Returns a `NodeViewMut` for the given node ID, if it exists.
    pub fn view_mut(&mut self, id: NodeId) -> Option<NodeViewMut<'_>> {
        if self.node(id).is_some() {
            Some(NodeViewMut::new(self, id))
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
        if self.node(id).is_some() {
            Some(NodeViewMut::new(self, id).classify())
        } else {
            None
        }
    }

    /// Looks up a node by path and returns an immutable classified view.
    pub fn get_by_path(&self, path: &str) -> Option<NodeType<'_>> {
        let id = self.get_by_path_id(path)?;
        Some(NodeView::new(self, id).classify())
    }

    /// Looks up a node by path and returns a mutable classified view.
    pub fn get_by_path_mut(&mut self, path: &str) -> Option<NodeTypeMut<'_>> {
        let id = self.get_by_path_id(path)?;
        Some(NodeViewMut::new(self, id).classify())
    }

    /// Looks up a node by phandle and returns an immutable classified view.
    pub fn get_by_phandle(&self, phandle: crate::Phandle) -> Option<NodeType<'_>> {
        let id = self.get_by_phandle_id(phandle)?;
        Some(NodeView::new(self, id).classify())
    }

    /// Looks up a node by phandle and returns a mutable classified view.
    pub fn get_by_phandle_mut(&mut self, phandle: crate::Phandle) -> Option<NodeTypeMut<'_>> {
        let id = self.get_by_phandle_id(phandle)?;
        Some(NodeViewMut::new(self, id).classify())
    }

    /// Returns a depth-first iterator over `NodeView`s.
    pub fn iter_raw_nodes(&self) -> impl Iterator<Item = NodeView<'_>> {
        self.iter_node_ids().map(move |id| NodeView::new(self, id))
    }

    /// Returns a depth-first iterator over classified `NodeType`s.
    pub fn all_nodes(&self) -> impl Iterator<Item = NodeType<'_>> {
        self.iter_raw_nodes().map(|v| v.classify())
    }
}
