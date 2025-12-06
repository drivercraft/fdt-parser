use alloc::vec::Vec;

use super::{NodeOp, NodeTrait, RawNode};
pub use fdt_raw::MemoryRegion;

/// Memory 节点，描述物理内存布局
#[derive(Clone, Debug)]
pub struct NodeMemory {
    raw: RawNode,
    pub regions: Vec<MemoryRegion>,
}

impl NodeOp for NodeMemory {}

impl NodeTrait for NodeMemory {
    fn as_raw(&self) -> &RawNode {
        &self.raw
    }

    fn as_raw_mut(&mut self) -> &mut RawNode {
        &mut self.raw
    }

    fn to_raw(self) -> RawNode {
        self.raw
    }
}

impl NodeMemory {
    pub fn new(name: &str) -> Self {
        NodeMemory {
            raw: RawNode::new(name),
            regions: Vec::new(),
        }
    }
}
