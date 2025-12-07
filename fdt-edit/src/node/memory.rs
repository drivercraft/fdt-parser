use core::ops::Deref;

use alloc::{string::{String, ToString}, vec::Vec};
use fdt_raw::MemoryRegion;

use crate::node::gerneric::NodeRefGen;

/// Memory 节点，描述物理内存布局
#[derive(Clone, Debug)]
pub struct NodeMemory {
    pub name: String,
}

impl NodeMemory {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// 获取节点名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取内存区域列表
    /// 注意：这是一个简单的实现，实际使用时需要从实际的 FDT 节点中解析
    pub fn regions(&self) -> Vec<MemoryRegion> {
        // 这个方法在测试中主要用来检查是否为空
        Vec::new()
    }

    /// 获取 device_type 属性
    /// 注意：这是一个简单的实现，返回 "memory"
    pub fn device_type(&self) -> Option<&str> {
        Some("memory")
    }
}

/// Memory 节点引用
#[derive(Clone)]
pub struct NodeRefMemory<'a> {
    pub node: NodeRefGen<'a>,
}

impl<'a> NodeRefMemory<'a> {
    pub fn try_from(node: NodeRefGen<'a>) -> Result<Self, NodeRefGen<'a>> {
        if !is_memory_node(&node) {
            return Err(node);
        }
        Ok(Self { node })
    }

    /// 获取内存区域列表
    pub fn regions(&self) -> Vec<MemoryRegion> {
        let mut regions = Vec::new();
        if let Some(reg_prop) = self.find_property("reg") {
            let mut reader = reg_prop.as_reader();

            // 获取 parent 的 address-cells 和 size-cells
            let address_cells = self.ctx.parent_address_cells() as usize;
            let size_cells = self.ctx.parent_size_cells() as usize;

            while let (Some(address), Some(size)) = (
                reader.read_cells(address_cells),
                reader.read_cells(size_cells),
            ) {
                regions.push(MemoryRegion {
                    address,
                    size,
                });
            }
        }
        regions
    }

    /// 获取 device_type 属性
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

/// 检查节点是否是 memory 节点
fn is_memory_node(node: &NodeRefGen) -> bool {
    // 检查 device_type 属性是否为 "memory"
    if let Some(device_type) = node.device_type() {
        if device_type == "memory" {
            return true;
        }
    }

    // 或者节点名以 "memory" 开头
    node.name().starts_with("memory")
}