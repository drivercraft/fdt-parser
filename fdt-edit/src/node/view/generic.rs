//! Generic node view specialization.

use alloc::{string::String, vec::Vec};
use fdt_raw::RegInfo;

use super::NodeView;
use crate::{NodeId, RegFixed, ViewOp};

// ---------------------------------------------------------------------------
// GenericNodeView
// ---------------------------------------------------------------------------

/// A generic node view with no extra specialization.
#[derive(Clone, Copy)]
pub struct NodeGeneric<'a> {
    pub(super) inner: NodeView<'a>,
}

impl<'a> NodeGeneric<'a> {
    pub fn id(&self) -> NodeId {
        self.inner.id()
    }

    pub fn path(&self) -> String {
        self.inner.path()
    }

    pub fn regs(&self) -> Vec<RegFixed> {
        self.inner.regs()
    }
}

impl<'a> ViewOp<'a> for NodeGeneric<'a> {
    fn as_view(&self) -> NodeView<'a> {
        self.inner
    }
}

// ---------------------------------------------------------------------------
// GenericNodeViewMut
// ---------------------------------------------------------------------------

/// Mutable view for generic nodes.
pub struct NodeGenericMut<'a> {
    pub(super) inner: NodeView<'a>,
}

impl<'a> ViewOp<'a> for NodeGenericMut<'a> {
    fn as_view(&self) -> NodeView<'a> {
        self.inner
    }
}

impl<'a> NodeGenericMut<'a> {
    pub fn id(&self) -> NodeId {
        self.inner.id()
    }

    pub fn path(&self) -> String {
        self.inner.path()
    }

    pub fn set_regs(&mut self, regs: &[RegInfo]) {
        self.inner.set_regs(regs);
    }
}
