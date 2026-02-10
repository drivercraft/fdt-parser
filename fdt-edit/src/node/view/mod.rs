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

use crate::{Fdt, Node, NodeId};

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

    fn classify(&self) -> NodeType<'a> {
        if let Some(node) = MemoryNodeView::try_from_view(*self) {
            return NodeType::Memory(node);
        }

        if let Some(node) = IntcNodeView::try_from_view(*self) {
            return NodeType::InterruptController(node);
        }

        NodeType::Generic(NodeGeneric { inner: *self })
    }

    fn classify_mut(&mut self) -> NodeTypeMut<'a> {
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

    /// Looks up a node by path and returns an immutable classified view.
    pub fn get_by_path(&self, path: &str) -> Option<NodeType<'_>> {
        let id = self.get_by_path_id(path)?;
        Some(NodeView::new(self, id).classify())
    }

    /// Looks up a node by path and returns a mutable classified view.
    pub fn get_by_path_mut(&mut self, path: &str) -> Option<NodeTypeMut<'_>> {
        let id = self.get_by_path_id(path)?;
        Some(NodeView::new(self, id).classify_mut())
    }

    /// Looks up a node by phandle and returns an immutable classified view.
    pub fn get_by_phandle(&self, phandle: crate::Phandle) -> Option<NodeType<'_>> {
        let id = self.get_by_phandle_id(phandle)?;
        Some(NodeView::new(self, id).classify())
    }

    /// Looks up a node by phandle and returns a mutable classified view.
    pub fn get_by_phandle_mut(&mut self, phandle: crate::Phandle) -> Option<NodeTypeMut<'_>> {
        let id = self.get_by_phandle_id(phandle)?;
        Some(NodeView::new(self, id).classify_mut())
    }

    /// Returns a depth-first iterator over `NodeView`s.
    fn iter_raw_nodes(&self) -> impl Iterator<Item = NodeView<'_>> {
        self.iter_node_ids().map(move |id| NodeView::new(self, id))
    }

    /// Returns a depth-first iterator over classified `NodeType`s.
    pub fn all_nodes(&self) -> impl Iterator<Item = NodeType<'_>> {
        self.iter_raw_nodes().map(|v| v.classify())
    }
}
