use core::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use crate::Node;

mod base;
mod iter_ref;
mod memory;

pub use base::*;
use enum_dispatch::enum_dispatch;
pub(crate) use iter_ref::*;
pub use memory::*;

#[enum_dispatch(NodeOp)]
pub enum NodeKind {
    Generic(NodeGeneric),
    Memory(NodeMemory),
}

#[enum_dispatch]
pub(crate) trait NodeOp {
    fn as_generic(&self) -> &NodeGeneric;
    fn as_generic_mut(&mut self) -> &mut NodeGeneric;
    fn _display(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_generic().fmt(f)
    }
}

impl NodeKind {
    pub(crate) fn new(node: *mut Node, meta: NodeIterMeta) -> Self {
        let generic = NodeGeneric::new(node, meta.clone());

        let generic = match NodeMemory::try_new(generic) {
            Ok(mem) => return NodeKind::Memory(mem),
            Err(generic) => generic,
        };

        NodeKind::Generic(generic)
    }

    pub(crate) fn _as_raw<'a>(&self) -> &'a Node {
        self.as_generic().as_node()
    }

    pub(crate) fn _as_raw_mut<'a>(&mut self) -> &'a mut Node {
        self.as_generic_mut().as_node_mut()
    }
}

impl NodeOp for NodeGeneric {
    fn as_generic(&self) -> &NodeGeneric {
        self
    }

    fn as_generic_mut(&mut self) -> &mut NodeGeneric {
        self
    }
}

pub struct NodeRef<'a> {
    inner: NodeKind,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> NodeRef<'a> {
    pub(crate) fn new(node: *mut Node, meta: NodeIterMeta) -> Self {
        Self {
            inner: NodeKind::new(node, meta),
            _marker: core::marker::PhantomData,
        }
    }

    pub fn as_raw(&self) -> &Node {
        self.inner._as_raw()
    }
}

impl<'a> Deref for NodeRef<'a> {
    type Target = NodeKind;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Display for NodeRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.inner._display(f)
    }
}

pub struct NodeRefMut<'a> {
    inner: NodeKind,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> NodeRefMut<'a> {
    pub(crate) fn new(node: *mut Node, meta: NodeIterMeta) -> Self {
        Self {
            inner: NodeKind::new(node, meta),
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a> Deref for NodeRefMut<'a> {
    type Target = NodeKind;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> DerefMut for NodeRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Display for NodeRefMut<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.inner._display(f)
    }
}
