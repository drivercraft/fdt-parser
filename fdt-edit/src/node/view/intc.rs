//! Interrupt controller node view specialization.

use core::ops::Deref;

use super::NodeView;
use crate::{NodeGeneric, NodeGenericMut, ViewOp};

// ---------------------------------------------------------------------------
// IntcNodeView
// ---------------------------------------------------------------------------

/// Specialized view for interrupt controller nodes.
#[derive(Clone, Copy)]
pub struct IntcNodeView<'a> {
    pub(super) inner: NodeGeneric<'a>,
}

impl<'a> ViewOp<'a> for IntcNodeView<'a> {
    fn as_view(&self) -> NodeView<'a> {
        self.inner.as_view()
    }
}

impl<'a> Deref for IntcNodeView<'a> {
    type Target = NodeGeneric<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> IntcNodeView<'a> {
    pub(crate) fn try_from_view(view: NodeView<'a>) -> Option<Self> {
        if view.as_node().is_interrupt_controller() {
            Some(Self {
                inner: NodeGeneric { inner: view },
            })
        } else {
            None
        }
    }

    /// Returns the `#interrupt-cells` property value.
    pub fn interrupt_cells(&self) -> Option<u32> {
        self.as_view().as_node().interrupt_cells()
    }

    /// This is always `true` for `IntcNodeView` (type-level guarantee).
    pub fn is_interrupt_controller(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// IntcNodeViewMut
// ---------------------------------------------------------------------------

/// Mutable view for interrupt controller nodes.
pub struct IntcNodeViewMut<'a> {
    pub(super) inner: NodeGenericMut<'a>,
}

impl<'a> ViewOp<'a> for IntcNodeViewMut<'a> {
    fn as_view(&self) -> NodeView<'a> {
        self.inner.as_view()
    }
}

impl<'a> Deref for IntcNodeViewMut<'a> {
    type Target = NodeGenericMut<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> IntcNodeViewMut<'a> {
    pub(crate) fn try_from_view(view: NodeView<'a>) -> Option<Self> {
        if view.as_node().is_interrupt_controller() {
            Some(Self {
                inner: NodeGenericMut { inner: view },
            })
        } else {
            None
        }
    }

    pub fn set_regs(&mut self, regs: &[fdt_raw::RegInfo]) {
        self.inner.set_regs(regs);
    }
}
