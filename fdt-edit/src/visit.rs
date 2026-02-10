//! Visitor traits for traversing device tree nodes.
//!
//! Inspired by `toml_edit::visit` / `visit_mut`. The `Visit` trait provides
//! immutable traversal with `NodeView`, while `VisitMut` provides mutable
//! traversal using `(&mut Fdt, NodeId)` pairs to avoid aliasing issues.

use alloc::vec::Vec;

use crate::{Fdt, NodeId, NodeView, Property};

// ---------------------------------------------------------------------------
// Visit — immutable traversal
// ---------------------------------------------------------------------------

/// Immutable visitor trait for device tree traversal.
///
/// Override individual methods to customize behavior. Default implementations
/// recurse through the tree structure.
///
/// # Example
///
/// ```ignore
/// struct NodeCounter { count: usize }
///
/// impl<'a> Visit<'a> for NodeCounter {
///     fn visit_node(&mut self, node: NodeView<'a>) {
///         self.count += 1;
///         visit_node(self, node); // recurse children
///     }
/// }
/// ```
pub trait Visit<'a> {
    /// Called for every node. Default: classifies and dispatches to typed methods.
    fn visit_node(&mut self, node: NodeView<'a>) {
        visit_node(self, node);
    }

    /// Called for memory nodes. Default: visits properties and recurses children.
    fn visit_memory_node(&mut self, node: NodeView<'a>) {
        visit_node_children(self, node);
    }

    /// Called for interrupt controller nodes. Default: visits properties and recurses children.
    fn visit_intc_node(&mut self, node: NodeView<'a>) {
        visit_node_children(self, node);
    }

    /// Called for generic nodes. Default: visits properties and recurses children.
    fn visit_generic_node(&mut self, node: NodeView<'a>) {
        visit_node_children(self, node);
    }

    /// Called for each property. Default: no-op.
    fn visit_property(&mut self, _node: NodeView<'a>, _prop: &'a Property) {}
}

/// Default `visit_node` implementation: classifies and dispatches.
pub fn visit_node<'a, V: Visit<'a> + ?Sized>(v: &mut V, node: NodeView<'a>) {
    let n = node.as_node();
    if n.is_interrupt_controller() {
        v.visit_intc_node(node);
    } else if n.is_memory() {
        v.visit_memory_node(node);
    } else {
        v.visit_generic_node(node);
    }
}

/// Visits properties then recurses into children (used by default typed methods).
pub fn visit_node_children<'a, V: Visit<'a> + ?Sized>(v: &mut V, node: NodeView<'a>) {
    for prop in node.properties() {
        v.visit_property(node, prop);
    }
    for child in node.children() {
        v.visit_node(child);
    }
}

// ---------------------------------------------------------------------------
// VisitMut — mutable traversal
// ---------------------------------------------------------------------------

/// Mutable visitor trait for device tree traversal.
///
/// Takes `(&mut Fdt, NodeId)` pairs instead of view types to avoid
/// `&mut` aliasing. The default implementations clone children lists
/// before recursing to avoid borrow conflicts.
pub trait VisitMut {
    /// Called for every node. Default: classifies and dispatches.
    fn visit_node_mut(&mut self, fdt: &mut Fdt, id: NodeId) {
        visit_node_mut(self, fdt, id);
    }

    /// Called for memory nodes.
    fn visit_memory_node_mut(&mut self, fdt: &mut Fdt, id: NodeId) {
        visit_node_children_mut(self, fdt, id);
    }

    /// Called for interrupt controller nodes.
    fn visit_intc_node_mut(&mut self, fdt: &mut Fdt, id: NodeId) {
        visit_node_children_mut(self, fdt, id);
    }

    /// Called for generic nodes.
    fn visit_generic_node_mut(&mut self, fdt: &mut Fdt, id: NodeId) {
        visit_node_children_mut(self, fdt, id);
    }

    /// Called for each property.
    fn visit_property_mut(&mut self, _fdt: &mut Fdt, _node_id: NodeId, _prop_name: &str) {}
}

/// Default `visit_node_mut`: classifies and dispatches.
pub fn visit_node_mut<V: VisitMut + ?Sized>(v: &mut V, fdt: &mut Fdt, id: NodeId) {
    let (is_intc, is_mem) = {
        match fdt.node(id) {
            Some(n) => (n.is_interrupt_controller(), n.is_memory()),
            None => return,
        }
    };

    if is_intc {
        v.visit_intc_node_mut(fdt, id);
    } else if is_mem {
        v.visit_memory_node_mut(fdt, id);
    } else {
        v.visit_generic_node_mut(fdt, id);
    }
}

/// Visits properties then recurses children (mutable default).
pub fn visit_node_children_mut<V: VisitMut + ?Sized>(v: &mut V, fdt: &mut Fdt, id: NodeId) {
    // Clone property names to avoid borrow conflict
    let prop_names: Vec<_> = match fdt.node(id) {
        Some(n) => n.properties().iter().map(|p| p.name.clone()).collect(),
        None => return,
    };

    for name in &prop_names {
        v.visit_property_mut(fdt, id, name);
    }

    // Clone children list to avoid borrow conflict
    let children: Vec<NodeId> = match fdt.node(id) {
        Some(n) => n.children().to_vec(),
        None => return,
    };

    for child_id in children {
        v.visit_node_mut(fdt, child_id);
    }
}

// ---------------------------------------------------------------------------
// Convenience entry points on Fdt
// ---------------------------------------------------------------------------

impl Fdt {
    /// Run an immutable visitor starting from the root.
    pub fn visit<'a, V: Visit<'a>>(&'a self, visitor: &mut V) {
        visitor.visit_node(self.root());
    }

    /// Run a mutable visitor starting from the root.
    pub fn visit_mut<V: VisitMut>(&mut self, visitor: &mut V) {
        let root = self.root_id();
        visitor.visit_node_mut(self, root);
    }
}
