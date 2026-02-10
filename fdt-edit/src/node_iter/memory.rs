use alloc::vec::Vec;
use fdt_raw::MemoryRegion;

use crate::{NodeGeneric, NodeOp};

pub struct NodeMemory {
    pub(crate) base: NodeGeneric,
}

impl NodeMemory {
    pub(crate) fn try_new(node: NodeGeneric) -> Result<Self, NodeGeneric> {
        if is_memory_node(&node) {
            Ok(Self { base: node })
        } else {
            Err(node)
        }
    }
}

impl NodeOp for NodeMemory {
    fn as_generic(&self) -> &NodeGeneric {
        &self.base
    }

    fn as_generic_mut(&mut self) -> &mut NodeGeneric {
        &mut self.base
    }
}

/// Check if node is a memory node
fn is_memory_node(node: &NodeGeneric) -> bool {
    // Check if device_type property is "memory"
    if let Some(device_type) = node.as_node().device_type()
        && device_type == "memory"
    {
        return true;
    }

    // Or node name starts with "memory"
    node.as_node().name().starts_with("memory")
}
