use core::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use crate::Node;

mod base;
mod iter_ref;

pub use base::*;
pub(crate) use iter_ref::*;

pub enum NodeKind {
    Generic(NodeGeneric),
}

impl NodeKind {
    pub(crate) fn new(node: *mut Node, meta: NodeIterMeta) -> Self {
        NodeKind::Generic(NodeGeneric::new(node, meta))
    }

    pub(crate) fn _as_raw<'a>(&self) -> &'a Node {
        match self {
            NodeKind::Generic(generic) => generic.as_node(),
        }
    }

    pub(crate) fn _as_raw_mut<'a>(&mut self) -> &'a mut Node {
        match self {
            NodeKind::Generic(generic) => generic.as_node_mut(),
        }
    }

    pub(crate) fn _fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NodeKind::Generic(generic) => generic.fmt(f),
        }
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
        self.inner._fmt(f)
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
        self.inner._fmt(f)
    }
}
