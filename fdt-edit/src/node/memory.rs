use core::ops::Deref;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use fdt_raw::MemoryRegion;

use crate::node::gerneric::NodeRefGen;

/// Memory node describing physical memory layout.
#[derive(Clone, Debug)]
pub struct NodeMemory {
    /// Node name
    pub name: String,
}

impl NodeMemory {
    /// Creates a new memory node with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Get node name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get memory region list
    /// Note: This is a simple implementation, in actual use needs to parse from real FDT nodes
    pub fn regions(&self) -> Vec<MemoryRegion> {
        // This method is mainly used in tests to check if empty
        Vec::new()
    }

    /// Get device_type property
    /// Note: This is a simple implementation, returns "memory"
    pub fn device_type(&self) -> Option<&str> {
        Some("memory")
    }
}

/// Memory node reference.
///
/// Provides specialized access to memory nodes and their regions.
#[derive(Clone)]
pub struct NodeRefMemory<'a> {
    /// The underlying generic node reference
    pub node: NodeRefGen<'a>,
}

impl<'a> NodeRefMemory<'a> {
    /// Attempts to create a memory node reference from a generic node.
    ///
    /// Returns `Err` with the original node if it's not a memory node.
    pub fn try_from(node: NodeRefGen<'a>) -> Result<Self, NodeRefGen<'a>> {
        if !is_memory_node(&node) {
            return Err(node);
        }
        Ok(Self { node })
    }

    /// Get memory region list
    pub fn regions(&self) -> Vec<MemoryRegion> {
        let mut regions = Vec::new();
        if let Some(reg_prop) = self.find_property("reg") {
            let mut reader = reg_prop.as_reader();

            // Get parent's address-cells and size-cells
            let address_cells = self.ctx.parent_address_cells() as usize;
            let size_cells = self.ctx.parent_size_cells() as usize;

            while let (Some(address), Some(size)) = (
                reader.read_cells(address_cells),
                reader.read_cells(size_cells),
            ) {
                regions.push(MemoryRegion { address, size });
            }
        }
        regions
    }

    /// Get device_type property
    pub fn device_type(&self) -> Option<&str> {
        self.find_property("device_type")
            .and_then(|prop| prop.as_str())
    }
}

impl<'a> Deref for NodeRefMemory<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

/// Check if node is a memory node
fn is_memory_node(node: &NodeRefGen) -> bool {
    // Check if device_type property is "memory"
    if let Some(device_type) = node.device_type()
        && device_type == "memory"
    {
        return true;
    }

    // Or node name starts with "memory"
    node.name().starts_with("memory")
}
