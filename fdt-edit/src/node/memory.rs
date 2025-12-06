use alloc::vec::Vec;

use super::{NodeOp, NodeTrait, RawNode};
pub use fdt_raw::MemoryRegion;

/// Memory 节点，描述物理内存布局
#[derive(Clone, Debug)]
pub struct NodeMemory {
    raw: RawNode,
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
        }
    }

    pub fn from_raw(raw: RawNode) -> Self {
        NodeMemory { raw }
    }

    /// 获取内存区域列表
    pub fn regions(&self) -> Vec<MemoryRegion> {
        let mut regions = Vec::new();
        if let Some(reg_prop) = self.raw.find_property("reg")
            && let crate::prop::PropertyKind::Reg(regs) = &reg_prop.kind
        {
            for reg in regs {
                regions.push(MemoryRegion {
                    address: reg.address,
                    size: reg.size.unwrap_or(0),
                });
            }
        }
        regions
    }

    /// 获取 device_type 属性
    pub fn device_type(&self) -> Option<&str> {
        self.raw
            .find_property("device_type")
            .and_then(|p| match &p.kind {
                crate::prop::PropertyKind::Str(s) => Some(s.as_str()),
                _ => None,
            })
    }
}
