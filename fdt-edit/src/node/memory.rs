use alloc::vec::Vec;

use super::{NodeOp, NodeTrait, RawNode};
use crate::prop::PropertyKind;
pub use fdt_raw::MemoryRegion;

/// Memory 节点，描述物理内存布局
#[derive(Clone, Debug)]
pub struct NodeMemory(pub(crate) RawNode);

impl NodeOp for NodeMemory {}

impl NodeTrait for NodeMemory {
    fn as_raw(&self) -> &RawNode {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawNode {
        &mut self.0
    }

    fn to_raw(self) -> RawNode {
        self.0
    }
}

impl NodeMemory {
    pub fn new(name: &str) -> Self {
        NodeMemory(RawNode::new(name))
    }

    /// 获取内存区域列表
    ///
    /// Memory 节点的 reg 属性描述了物理内存的布局
    pub fn regions(&self) -> Vec<MemoryRegion> {
        let Some(prop) = self.find_property("reg") else {
            return Vec::new();
        };

        let PropertyKind::Reg(entries) = &prop.kind else {
            return Vec::new();
        };

        entries
            .iter()
            .map(|entry| MemoryRegion {
                address: entry.address,
                size: entry.size.unwrap_or(0),
            })
            .collect()
    }

    /// 计算总内存大小
    pub fn total_size(&self) -> u64 {
        self.regions().iter().map(|r| r.size).sum()
    }
}
