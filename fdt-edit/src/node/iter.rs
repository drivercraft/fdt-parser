use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice::Iter,
};

use alloc::vec::Vec;

use crate::{
    Context, Node, NodeKind, NodeRefClock, NodeRefInterruptController, NodeRefMemory, NodeRefPci,
    node::gerneric::{NodeMutGen, NodeRefGen},
};

/// Enum representing a reference to a specialized node type.
///
/// This enum provides automatic type detection and dispatch for different
/// node types based on their properties and compatible strings.
#[derive(Clone)]
pub enum NodeRef<'a> {
    /// Generic node without specific type
    Gerneric(NodeRefGen<'a>),
    /// PCI bridge node
    Pci(NodeRefPci<'a>),
    /// Clock provider node
    Clock(NodeRefClock<'a>),
    /// Interrupt controller node
    InterruptController(NodeRefInterruptController<'a>),
    /// Memory reservation node
    Memory(NodeRefMemory<'a>),
}

impl<'a> NodeRef<'a> {
    /// Creates a new node reference with automatic type detection.
    ///
    /// Attempts to create specialized references (PCI, Clock, etc.) based on
    /// the node's properties and compatible strings.
    pub fn new(node: &'a Node, ctx: Context<'a>) -> Self {
        let mut g = NodeRefGen { node, ctx };

        // Try PCI first
        g = match NodeRefPci::try_from(g) {
            Ok(pci) => return Self::Pci(pci),
            Err(v) => v,
        };

        // Then try Clock
        g = match NodeRefClock::try_from(g) {
            Ok(clock) => return Self::Clock(clock),
            Err(v) => v,
        };

        // Then try InterruptController
        g = match NodeRefInterruptController::try_from(g) {
            Ok(ic) => return Self::InterruptController(ic),
            Err(v) => v,
        };

        // Finally try Memory
        g = match NodeRefMemory::try_from(g) {
            Ok(mem) => return Self::Memory(mem),
            Err(v) => v,
        };

        Self::Gerneric(g)
    }

    /// Get concrete node type for pattern matching
    pub fn as_ref(&self) -> NodeKind<'a> {
        match self {
            NodeRef::Clock(clock) => NodeKind::Clock(clock.clone()),
            NodeRef::Pci(pci) => NodeKind::Pci(pci.clone()),
            NodeRef::InterruptController(ic) => NodeKind::InterruptController(ic.clone()),
            NodeRef::Memory(mem) => NodeKind::Memory(mem.clone()),
            NodeRef::Gerneric(generic) => NodeKind::Generic(generic.clone()),
        }
    }
}

impl<'a> Deref for NodeRef<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            NodeRef::Gerneric(n) => n,
            NodeRef::Pci(n) => &n.node,
            NodeRef::Clock(n) => &n.node,
            NodeRef::InterruptController(n) => &n.node,
            NodeRef::Memory(n) => &n.node,
        }
    }
}

/// Enum representing a mutable reference to a node.
///
/// Currently only generic mutable nodes are supported.
pub enum NodeMut<'a> {
    /// Generic mutable node reference
    Gerneric(NodeMutGen<'a>),
}

impl<'a> NodeMut<'a> {
    /// Creates a new mutable node reference.
    pub fn new(node: &'a mut Node, ctx: Context<'a>) -> Self {
        Self::Gerneric(NodeMutGen { node, ctx })
    }
}

impl<'a> Deref for NodeMut<'a> {
    type Target = NodeMutGen<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            NodeMut::Gerneric(n) => n,
        }
    }
}

impl<'a> DerefMut for NodeMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            NodeMut::Gerneric(n) => n,
        }
    }
}

/// Iterator over nodes in a device tree.
///
/// Provides depth-first traversal with automatic type detection for each node.
pub struct NodeIter<'a> {
    ctx: Context<'a>,
    node: Option<&'a Node>,
    stack: Vec<Iter<'a, Node>>,
}

impl<'a> NodeIter<'a> {
    /// Creates a new node iterator starting from the root node.
    pub fn new(root: &'a Node) -> Self {
        let mut ctx = Context::new();
        // Build phandle_map for entire tree upfront
        // This allows finding any node by phandle during traversal
        Context::build_phandle_map_from_node(root, &mut ctx.phandle_map);

        Self {
            ctx,
            node: Some(root),
            stack: vec![],
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node.take() {
            // Return current node and push its children onto stack
            let ctx = self.ctx.clone();
            self.ctx.push(n);
            self.stack.push(n.children.iter());
            return Some(NodeRef::new(n, ctx));
        }

        let iter = self.stack.last_mut()?;

        if let Some(child) = iter.next() {
            // Return child node and push its children onto stack
            let ctx = self.ctx.clone();
            self.ctx.push(child);
            self.stack.push(child.children.iter());
            return Some(NodeRef::new(child, ctx));
        }

        // Current iterator exhausted, pop from stack
        self.stack.pop();
        self.ctx.parents.pop();
        self.next()
    }
}

/// Mutable iterator over nodes in a device tree.
///
/// Provides depth-first traversal with mutable access to nodes.
pub struct NodeIterMut<'a> {
    ctx: Context<'a>,
    node: Option<NonNull<Node>>,
    stack: Vec<RawChildIter>,
    _marker: core::marker::PhantomData<&'a mut Node>,
}

/// Raw pointer-based child node iterator.
///
/// Used internally by `NodeIterMut` to avoid borrow conflicts.
struct RawChildIter {
    ptr: *mut Node,
    end: *mut Node,
}

impl RawChildIter {
    fn new(children: &mut Vec<Node>) -> Self {
        let ptr = children.as_mut_ptr();
        let end = unsafe { ptr.add(children.len()) };
        Self { ptr, end }
    }

    fn next(&mut self) -> Option<NonNull<Node>> {
        if self.ptr < self.end {
            let current = self.ptr;
            self.ptr = unsafe { self.ptr.add(1) };
            NonNull::new(current)
        } else {
            None
        }
    }
}

impl<'a> NodeIterMut<'a> {
    /// Creates a new mutable node iterator starting from the root node.
    pub fn new(root: &'a mut Node) -> Self {
        let mut ctx = Context::new();
        // Build phandle_map for entire tree upfront
        // Use raw pointers to avoid borrow conflicts
        let root_ptr = root as *mut Node;
        unsafe {
            // Build phandle_map using immutable reference
            Context::build_phandle_map_from_node(&*root_ptr, &mut ctx.phandle_map);
        }

        Self {
            ctx,
            node: NonNull::new(root_ptr),
            stack: vec![],
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for NodeIterMut<'a> {
    type Item = NodeMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node_ptr) = self.node.take() {
            // Return current node and push its children onto stack
            let ctx = self.ctx.clone();
            unsafe {
                let node_ref = node_ptr.as_ref();
                self.ctx.push(node_ref);
                let node_mut = &mut *node_ptr.as_ptr();
                self.stack.push(RawChildIter::new(&mut node_mut.children));
                return Some(NodeMut::new(node_mut, ctx));
            }
        }

        let iter = self.stack.last_mut()?;

        if let Some(child_ptr) = iter.next() {
            // Return child node and push its children onto stack
            let ctx = self.ctx.clone();
            unsafe {
                let child_ref = child_ptr.as_ref();
                self.ctx.push(child_ref);
                let child_mut = &mut *child_ptr.as_ptr();
                self.stack.push(RawChildIter::new(&mut child_mut.children));
                return Some(NodeMut::new(child_mut, ctx));
            }
        }

        // Current iterator exhausted, pop from stack
        self.stack.pop();
        self.ctx.parents.pop();
        self.next()
    }
}
