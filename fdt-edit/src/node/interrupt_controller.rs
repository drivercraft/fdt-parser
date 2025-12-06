use super::{NodeOp, NodeTrait, RawNode};
use crate::prop::PropertyKind;

/// 中断控制器节点
#[derive(Clone, Debug)]
pub struct NodeInterruptController(pub(crate) RawNode);

impl NodeOp for NodeInterruptController {}

impl NodeTrait for NodeInterruptController {
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

impl NodeInterruptController {
    pub fn try_from_raw(raw: RawNode) -> Result<Self, RawNode> {
        if !is_interrupt_controller_node(&raw) {
            return Err(raw);
        }
        Ok(NodeInterruptController(raw))
    }

    pub fn new(raw: RawNode) -> Self {
        NodeInterruptController(raw)
    }

    /// 获取 #interrupt-cells 值
    ///
    /// 这决定了引用此控制器的中断需要多少个 cell 来描述
    pub fn interrupt_cells(&self) -> Option<u32> {
        let prop = self.find_property("#interrupt-cells")?;
        if let PropertyKind::Num(v) = &prop.kind {
            Some(*v as u32)
        } else {
            None
        }
    }

    /// 获取 #address-cells 值（用于 interrupt-map）
    pub fn interrupt_address_cells(&self) -> Option<u32> {
        let prop = self.find_property("#address-cells")?;
        if let PropertyKind::Num(v) = &prop.kind {
            Some(*v as u32)
        } else {
            None
        }
    }

    /// 检查是否是中断控制器
    pub fn is_interrupt_controller(&self) -> bool {
        // 检查 interrupt-controller 属性（空属性标记）
        self.find_property("interrupt-controller").is_some()
    }
}

/// 检查节点是否是中断控制器
pub fn is_interrupt_controller_node(node: &RawNode) -> bool {
    // 名称以 interrupt-controller 开头
    if node.name.starts_with("interrupt-controller") {
        return true;
    }

    // 或者有 interrupt-controller 属性
    node.properties
        .iter()
        .any(|p| p.name() == "interrupt-controller")
}
