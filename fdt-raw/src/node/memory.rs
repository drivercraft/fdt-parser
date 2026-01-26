//! Memory node type for physical memory layout.
//!
//! This module provides the `Memory` type which represents memory nodes
//! in the device tree, describing the physical memory layout of the system.

use core::ops::Deref;

use super::NodeBase;

/// Memory region information.
///
/// Represents a contiguous region of physical memory with its base address
/// and size.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// Base address of the memory region
    pub address: u64,
    /// Size of the memory region in bytes
    pub size: u64,
}

/// Memory node describing physical memory layout.
///
/// This node type represents memory nodes in the device tree, which describe
/// the physical memory layout available to the system. The `reg` property
/// contains one or more memory regions.
#[derive(Clone)]
pub struct Memory<'a> {
    node: NodeBase<'a>,
}

impl<'a> Memory<'a> {
    /// Creates a new Memory wrapper from a NodeBase.
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        Self { node }
    }

    /// Returns an iterator over memory regions.
    ///
    /// The `reg` property of a memory node describes the physical memory
    /// layout, with each entry specifying a base address and size.
    pub fn regions(&self) -> impl Iterator<Item = MemoryRegion> + 'a {
        self.node.reg().into_iter().flat_map(|reg| {
            reg.map(|info| MemoryRegion {
                address: info.address,
                size: info.size.unwrap_or(0),
            })
        })
    }

    /// Returns all memory regions as a fixed-size array.
    ///
    /// This is useful for no_std environments where heap allocation is not
    /// available. Returns a `heapless::Vec` with at most N entries.
    pub fn regions_array<const N: usize>(&self) -> heapless::Vec<MemoryRegion, N> {
        let mut result = heapless::Vec::new();
        for region in self.regions() {
            if result.push(region).is_err() {
                break;
            }
        }
        result
    }

    /// Returns the total memory size across all regions.
    pub fn total_size(&self) -> u64 {
        self.regions().map(|r| r.size).sum()
    }
}

impl<'a> Deref for Memory<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl core::fmt::Debug for Memory<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut st = f.debug_struct("Memory");
        st.field("name", &self.node.name());
        for region in self.regions() {
            st.field("region", &region);
        }
        st.finish()
    }
}
