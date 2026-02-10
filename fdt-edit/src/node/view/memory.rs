//! Memory node view specialization.

use core::ops::Deref;

use alloc::vec::Vec;
use fdt_raw::MemoryRegion;

use super::NodeView;
use crate::{Node, NodeGeneric, NodeGenericMut, NodeId, ViewOp};

// ---------------------------------------------------------------------------
// MemoryNodeView
// ---------------------------------------------------------------------------

/// Specialized view for memory nodes.
///
/// Provides methods for parsing `reg` into memory regions.
#[derive(Clone, Copy)]
pub struct MemoryNodeView<'a> {
    pub(super) inner: NodeGeneric<'a>,
}

impl<'a> Deref for MemoryNodeView<'a> {
    type Target = NodeGeneric<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// Implement ViewOp for all specialized view types that have `inner: NodeView<'a>`
impl<'a> ViewOp<'a> for MemoryNodeView<'a> {
    fn as_view(&self) -> NodeView<'a> {
        self.inner.as_view()
    }
}

impl<'a> MemoryNodeView<'a> {
    pub(crate) fn try_from_view(view: NodeView<'a>) -> Option<Self> {
        if view.as_node().is_memory() {
            Some(Self {
                inner: NodeGeneric { inner: view },
            })
        } else {
            None
        }
    }

    /// Iterates over memory regions parsed from the `reg` property.
    ///
    /// Uses the parent node's `#address-cells` and `#size-cells` to decode.
    pub fn regions(&self) -> Vec<MemoryRegion> {
        let node = self.as_view().as_node();
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
        if let Some(parent) = self.as_view().parent() {
            let ac = parent.as_view().address_cells().unwrap_or(2) as usize;
            let sc = parent.as_view().size_cells().unwrap_or(1) as usize;
            (ac, sc)
        } else {
            (2, 1)
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryNodeViewMut
// ---------------------------------------------------------------------------

/// Mutable view for memory nodes.
pub struct MemoryNodeViewMut<'a> {
    pub(super) inner: NodeGenericMut<'a>,
}

impl<'a> Deref for MemoryNodeViewMut<'a> {
    type Target = NodeGenericMut<'a>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> ViewOp<'a> for MemoryNodeViewMut<'a> {
    fn as_view(&self) -> NodeView<'a> {
        self.inner.as_view()
    }
}

impl<'a> MemoryNodeViewMut<'a> {
    pub(crate) fn try_from_view(view: NodeView<'a>) -> Option<Self> {
        if view.as_node().is_memory() {
            Some(Self {
                inner: NodeGenericMut { inner: view },
            })
        } else {
            None
        }
    }
}
