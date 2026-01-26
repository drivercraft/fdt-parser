use core::ops::Deref;

use alloc::vec::Vec;

use crate::node::gerneric::NodeRefGen;

/// Interrupt controller node reference.
///
/// Provides specialized access to interrupt controller nodes and their properties.
#[derive(Clone)]
pub struct NodeRefInterruptController<'a> {
    /// The underlying generic node reference
    pub node: NodeRefGen<'a>,
}

impl<'a> NodeRefInterruptController<'a> {
    /// Attempts to create an interrupt controller reference from a generic node.
    ///
    /// Returns `Err` with the original node if it's not an interrupt controller.
    pub fn try_from(node: NodeRefGen<'a>) -> Result<Self, NodeRefGen<'a>> {
        if !is_interrupt_controller_node(&node) {
            return Err(node);
        }
        Ok(Self { node })
    }

    /// 获取 #interrupt-cells 值
    ///
    /// 这决定了引用此控制器的中断需要多少个 cell 来描述
    pub fn interrupt_cells(&self) -> Option<u32> {
        self.find_property("#interrupt-cells")
            .and_then(|prop| prop.get_u32())
    }

    /// 获取 #address-cells 值（用于 interrupt-map）
    pub fn interrupt_address_cells(&self) -> Option<u32> {
        self.find_property("#address-cells")
            .and_then(|prop| prop.get_u32())
    }

    /// 检查是否是中断控制器
    pub fn is_interrupt_controller(&self) -> bool {
        // 检查 interrupt-controller 属性（空属性标记）
        self.find_property("interrupt-controller").is_some()
    }

    /// 获取 compatible 列表
    pub fn compatibles(&self) -> Vec<&str> {
        self.node.compatibles().collect()
    }
}

impl<'a> Deref for NodeRefInterruptController<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

/// 检查节点是否是中断控制器
fn is_interrupt_controller_node(node: &NodeRefGen) -> bool {
    // 名称以 interrupt-controller 开头
    if node.name().starts_with("interrupt-controller") {
        return true;
    }

    // 或者有 interrupt-controller 属性
    node.find_property("interrupt-controller").is_some()
}
