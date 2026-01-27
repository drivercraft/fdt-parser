//! Memory node type.
//!
//! This module provides the `Memory` type for memory device nodes that
//! describe the physical memory layout of the system.

use core::{iter, ops::Deref};

use crate::{base::NodeBase, FdtError, MemoryRegion};

/// A memory device node.
///
/// Memory device nodes describe the physical memory layout for the system.
/// A system can have multiple memory nodes, or multiple memory ranges
/// specified in the `reg` property of a single memory node.
#[derive(Clone)]
pub struct Memory<'a> {
    node: NodeBase<'a>,
}

impl<'a> Memory<'a> {
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        Memory { node }
    }

    /// Returns an iterator over the memory regions described by this node.
    ///
    /// A memory device node is required for all devicetrees and describes the
    /// physical memory layout for the system. If a system has multiple ranges
    /// of memory, multiple memory nodes can be created, or the ranges can be
    /// specified in the reg property of a single memory node.
    pub fn regions(&self) -> impl Iterator<Item = Result<MemoryRegion, FdtError>> + 'a {
        let mut reg = self.node.reg();
        let mut has_error = false;
        iter::from_fn(move || {
            if has_error {
                return None;
            }
            match &mut reg {
                Ok(iter) => {
                    let one = iter.next()?;
                    Some(Ok(MemoryRegion {
                        address: one.address as usize as _,
                        size: one.size.unwrap_or_default(),
                    }))
                }
                Err(e) => {
                    has_error = true;
                    Some(Err(e.clone()))
                }
            }
        })
    }

    /// Get the name of this memory node.
    pub fn name(&self) -> &'a str {
        self.node.name()
    }
}

impl core::fmt::Debug for Memory<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut st = f.debug_struct("Memory");
        st.field("name", &self.name());
        for r in self.regions().flatten() {
            st.field("region", &r);
        }
        st.finish()
    }
}

impl<'a> Deref for Memory<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
