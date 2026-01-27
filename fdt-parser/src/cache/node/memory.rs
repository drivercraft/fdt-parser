use core::{fmt::Debug, ops::Deref};

use crate::{cache::node::NodeBase, FdtError, MemoryRegion};
use alloc::vec::Vec;

/// A memory node (cached version).
#[derive(Clone)]
pub struct Memory {
    node: NodeBase,
}

impl Memory {
    pub(crate) fn new(node: NodeBase) -> Self {
        Memory { node }
    }

    /// Get the memory regions defined by this memory node
    pub fn regions(&self) -> Result<Vec<MemoryRegion>, FdtError> {
        let reg = self.node.reg()?;
        let mut out = Vec::new();
        for r in reg {
            out.push(MemoryRegion {
                address: r.address as usize as _,
                size: r.size.unwrap_or_default(),
            });
        }
        Ok(out)
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Memory")
            .field("name", &self.node.name())
            .finish()
    }
}

impl Deref for Memory {
    type Target = NodeBase;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
