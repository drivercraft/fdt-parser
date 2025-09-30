use core::ops::Deref;

use super::Fdt;
use crate::{
    base::NodeIter,
    data::{Buffer, Raw, U32Iter2D},
    property::PropIter,
    FdtError, FdtRangeSilce, FdtReg, Phandle, Property, Status,
};

mod chosen;
mod clock;
mod interrupt_controller;
mod memory;

pub use chosen::*;
pub use clock::*;
pub use interrupt_controller::*;
pub use memory::*;

#[derive(Clone)]
pub struct NodeBase<'a> {
    name: &'a str,
    pub(crate) fdt: Fdt<'a>,
    pub level: usize,
    pub(crate) raw: Raw<'a>,
    pub(crate) parent: Option<ParentInfo<'a>>,
    interrupt_parent: Option<Phandle>,
}

#[derive(Clone)]
pub(crate) struct ParentInfo<'a> {
    pub name: &'a str,
    pub level: usize,
    pub raw: Raw<'a>,
    pub parent_address_cell: Option<u8>,
    pub parent_size_cell: Option<u8>,
    pub parent_name: Option<&'a str>,
}

impl<'a> NodeBase<'a> {
    pub(crate) fn new(
        name: &'a str,
        fdt: Fdt<'a>,
        raw: Raw<'a>,
        level: usize,
        parent: Option<&NodeBase<'a>>,
        interrupt_parent: Option<Phandle>,
    ) -> Self {
        let name = if name.is_empty() { "/" } else { name };
        NodeBase {
            name,
            fdt,
            level,
            parent: parent.map(|p| {
                let pp = p.parent_fast();

                let parent_address_cell = pp.as_ref().and_then(|pn| {
                    pn.find_property("#address-cells")
                        .ok()
                        .flatten()
                        .and_then(|prop| prop.u32().ok())
                        .map(|v| v as u8)
                });
                let parent_size_cell = pp.as_ref().and_then(|pn| {
                    pn.find_property("#size-cells")
                        .ok()
                        .flatten()
                        .and_then(|prop| prop.u32().ok())
                        .map(|v| v as u8)
                });

                ParentInfo {
                    name: p.name(),
                    level: p.level(),
                    raw: p.raw(),
                    parent_address_cell,
                    parent_size_cell,
                    parent_name: pp.as_ref().and_then(|pn| pn.parent_name()),
                }
            }),
            interrupt_parent,
            raw,
        }
    }

    pub fn parent_name(&self) -> Option<&'a str> {
        self.parent_fast().map(|p| p.name())
    }

    pub fn parent(&self) -> Option<Node<'a>> {
        let parent_name = self.parent_name()?;
        self.fdt
            .all_nodes()
            .flatten()
            .find(|node| node.name() == parent_name)
    }

    fn parent_fast(&self) -> Option<NodeBase<'a>> {
        self.parent.as_ref().map(|p| NodeBase {
            name: p.name,
            fdt: self.fdt.clone(),
            level: p.level,
            raw: p.raw,
            parent: None,
            interrupt_parent: None,
        })
    }

    pub fn raw(&self) -> Raw<'a> {
        self.raw
    }

    /// Get the name of this node
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Get the level/depth of this node in the device tree
    pub fn level(&self) -> usize {
        self.level
    }

    /// Get compatible strings for this node (placeholder implementation)
    pub fn compatibles(&self) -> Result<Option<impl Iterator<Item = &'a str> + 'a>, FdtError> {
        let prop = self.find_property("compatible")?;
        Ok(prop.map(|p| p.str_list()))
    }

    pub fn compatibles_flatten(&self) -> impl Iterator<Item = &'a str> + 'a {
        self.compatibles().ok().flatten().into_iter().flatten()
    }

    pub fn reg(&self) -> Result<Option<RegIter<'a>>, FdtError> {
        let pp_address_cell = self.parent.as_ref().and_then(|p| p.parent_address_cell);

        let prop = none_ok!(self.find_property("reg")?);

        // Use full parent resolution to retain its ancestor chain
        let parent = self.parent_fast().ok_or(FdtError::NodeNotFound("parent"))?;

        // reg parsing uses the immediate parent's cells
        let address_cell = parent
            .find_property("#address-cells")?
            .ok_or(FdtError::PropertyNotFound("#address-cells"))?
            .u32()? as u8;

        let size_cell = parent
            .find_property("#size-cells")?
            .ok_or(FdtError::PropertyNotFound("#size-cells"))?
            .u32()? as u8;

        let ranges = parent.node_ranges(pp_address_cell)?;

        Ok(Some(RegIter {
            size_cell,
            address_cell,
            buff: prop.data.buffer(),
            ranges,
        }))
    }

    fn is_interrupt_controller(&self) -> bool {
        let Ok(prop) = self.find_property("#interrupt-controller") else {
            return false;
        };
        prop.is_some()
    }

    /// 检查这个节点是否是根节点
    pub fn is_root(&self) -> bool {
        self.level == 0
    }

    /// 获取节点的完整路径信息（仅限调试用途）
    pub fn debug_info(&self) -> NodeDebugInfo<'a> {
        NodeDebugInfo {
            name: self.name(),
            level: self.level,
            pos: self.raw.pos(),
        }
    }

    pub fn properties(&self) -> impl Iterator<Item = Result<Property<'a>, FdtError>> + '_ {
        let reader = self.raw.buffer();
        PropIter::new(self.fdt.clone(), reader)
    }

    pub fn find_property(&self, name: &str) -> Result<Option<Property<'a>>, FdtError> {
        for prop in self.properties() {
            let prop = prop?;
            if prop.name.eq(name) {
                return Ok(Some(prop));
            }
        }
        Ok(None)
    }

    pub(crate) fn node_ranges(
        &self,
        address_cell_parent: Option<u8>,
    ) -> Result<Option<FdtRangeSilce<'a>>, FdtError> {
        let prop = none_ok!(self.find_property("ranges")?);

        let address_cell = self
            .find_property("#address-cells")?
            .ok_or(FdtError::PropertyNotFound("#address-cells"))?
            .u32()? as u8;
        let size_cell = self
            .find_property("#size-cells")?
            .ok_or(FdtError::PropertyNotFound("#size-cells"))?
            .u32()? as u8;
        let address_cell_parent = address_cell_parent
            .ok_or(FdtError::PropertyNotFound("parent.parent.#address-cells"))?;

        Ok(Some(FdtRangeSilce::new(
            address_cell,
            address_cell_parent,
            size_cell,
            prop.data.buffer(),
        )))
    }

    pub fn phandle(&self) -> Result<Option<Phandle>, FdtError> {
        let prop = self.find_property("phandle")?;
        match prop {
            Some(p) => Ok(Some(p.u32()?.into())),
            None => Ok(None),
        }
    }

    /// Find [InterruptController] from current node or its parent
    pub fn interrupt_parent(&self) -> Result<Option<InterruptController<'a>>, FdtError> {
        // First try to get the interrupt parent phandle from the node itself
        let phandle = match self.interrupt_parent {
            Some(p) => p,
            None => return Ok(None),
        };

        // Find the node with this phandle
        let node = self.fdt.get_node_by_phandle(phandle)?;
        let Some(node) = node else {
            return Ok(None);
        };
        match node {
            Node::InterruptController(ic) => Ok(Some(ic)),
            _ => Err(FdtError::NodeNotFound("interrupt-parent")),
        }
    }

    /// Get the interrupt parent phandle for this node
    pub fn get_interrupt_parent_phandle(&self) -> Option<Phandle> {
        self.interrupt_parent
    }

    pub fn interrupts(
        &self,
    ) -> Result<Option<impl Iterator<Item = impl Iterator<Item = u32> + 'a> + 'a>, FdtError> {
        let prop = none_ok!(self.find_property("interrupts")?);
        let irq_parent = self.interrupt_parent()?;
        let irq_parent = irq_parent.ok_or(FdtError::NodeNotFound("interrupt-parent"))?;
        let cell_size = irq_parent.interrupt_cells()?;
        let iter = U32Iter2D::new(&prop.data, cell_size);

        Ok(Some(iter))
    }

    pub fn clock_frequency(&self) -> Result<Option<u32>, FdtError> {
        let prop = none_ok!(self.find_property("clock-frequency")?);
        Ok(Some(prop.u32()?))
    }

    pub fn clocks(&self) -> Result<ClocksIter<'a>, FdtError> {
        ClocksIter::new(self)
    }

    pub fn children(&self) -> NodeChildIter<'a> {
        NodeChildIter {
            fdt: self.fdt.clone(),
            parent: self.clone(),
            all_nodes: None,
            target_level: 0,
            found_parent: false,
        }
    }

    pub fn status(&self) -> Result<Option<Status>, FdtError> {
        let prop = none_ok!(self.find_property("status")?);
        let s = prop.str()?;

        if s.contains("disabled") {
            return Ok(Some(Status::Disabled));
        }

        if s.contains("okay") {
            return Ok(Some(Status::Okay));
        }

        Ok(None)
    }
}

/// 节点调试信息
#[derive(Debug)]
pub struct NodeDebugInfo<'a> {
    pub name: &'a str,
    pub level: usize,
    pub pos: usize,
}

impl core::fmt::Debug for NodeBase<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Node").field("name", &self.name()).finish()
    }
}

pub struct RegIter<'a> {
    size_cell: u8,
    address_cell: u8,
    buff: Buffer<'a>,
    ranges: Option<FdtRangeSilce<'a>>,
}
impl Iterator for RegIter<'_> {
    type Item = FdtReg;

    fn next(&mut self) -> Option<Self::Item> {
        let child_bus_address = self.buff.take_by_cell_size(self.address_cell)?;

        let mut address = child_bus_address;

        if let Some(ranges) = &self.ranges {
            for one in ranges.iter() {
                let range_child_bus_address = one.child_bus_address().as_u64();
                let range_parent_bus_address = one.parent_bus_address().as_u64();

                if child_bus_address >= range_child_bus_address
                    && child_bus_address < range_child_bus_address + one.size
                {
                    address =
                        child_bus_address - range_child_bus_address + range_parent_bus_address;
                    break;
                }
            }
        }

        let size = if self.size_cell > 0 {
            Some(self.buff.take_by_cell_size(self.size_cell)? as usize)
        } else {
            None
        };
        Some(FdtReg {
            address,
            child_bus_address,
            size,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Node<'a> {
    General(NodeBase<'a>),
    Chosen(Chosen<'a>),
    Memory(Memory<'a>),
    InterruptController(InterruptController<'a>),
}

impl<'a> Node<'a> {
    pub fn node(&self) -> &NodeBase<'a> {
        self.deref()
    }
}

impl<'a> From<NodeBase<'a>> for Node<'a> {
    fn from(node: NodeBase<'a>) -> Self {
        if node.name() == "chosen" {
            Node::Chosen(Chosen::new(node))
        } else if node.name().starts_with("memory@") {
            Node::Memory(Memory::new(node))
        } else if node.is_interrupt_controller() {
            Node::InterruptController(InterruptController::new(node))
        } else {
            Node::General(node)
        }
    }
}

impl<'a> Deref for Node<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            Node::General(n) => n,
            Node::Chosen(n) => n,
            Node::Memory(n) => n,
            Node::InterruptController(n) => n,
        }
    }
}

pub struct NodeChildIter<'a> {
    fdt: Fdt<'a>,
    parent: NodeBase<'a>,
    all_nodes: Option<NodeIter<'a>>,
    target_level: usize,
    found_parent: bool,
}

impl<'a> Iterator for NodeChildIter<'a> {
    type Item = Result<Node<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        // 懒初始化节点迭代器
        if self.all_nodes.is_none() {
            self.all_nodes = Some(self.fdt.all_nodes());
        }

        let all_nodes = self.all_nodes.as_mut()?;

        // 寻找子节点
        loop {
            let node = match all_nodes.next()? {
                Ok(node) => node,
                Err(e) => return Some(Err(e)),
            };

            // 首先找到父节点
            if !self.found_parent {
                if node.name() == self.parent.name() && node.level() == self.parent.level() {
                    self.found_parent = true;
                    self.target_level = node.level() + 1;
                }
                continue;
            }

            // 已经找到父节点，现在查找子节点
            let current_level = node.level();

            // 如果当前节点的级别等于目标级别，并且在树结构中紧跟在父节点之后，
            // 那么它就是父节点的直接子节点
            if current_level == self.target_level {
                return Some(Ok(node));
            }

            // 如果当前节点的级别小于或等于父节点级别，说明我们已经离开了父节点的子树
            if current_level <= self.parent.level() {
                break;
            }
        }

        None
    }
}

impl<'a> NodeChildIter<'a> {
    /// 创建一个新的子节点迭代器
    pub fn new(fdt: Fdt<'a>, parent: NodeBase<'a>) -> Self {
        NodeChildIter {
            fdt,
            parent,
            all_nodes: None,
            target_level: 0,
            found_parent: false,
        }
    }

    /// 获取父节点的引用
    pub fn parent(&self) -> &NodeBase<'a> {
        &self.parent
    }

    /// 收集所有子节点到一个 Vec 中
    pub fn collect_children(self) -> Result<alloc::vec::Vec<Node<'a>>, FdtError> {
        self.collect()
    }

    /// 查找具有特定名称的子节点
    pub fn find_child_by_name(self, name: &str) -> Result<Option<Node<'a>>, FdtError> {
        for child_result in self {
            let child = child_result?;
            if child.name() == name {
                return Ok(Some(child));
            }
        }
        Ok(None)
    }

    /// 查找具有特定兼容性字符串的子节点
    pub fn find_child_by_compatible(self, compatible: &str) -> Result<Option<Node<'a>>, FdtError> {
        for child_result in self {
            let child = child_result?;
            if let Some(compatibles) = child.compatibles()? {
                for comp in compatibles {
                    if comp == compatible {
                        return Ok(Some(child));
                    }
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::Fdt;

    #[test]
    fn test_node_child_iter_basic() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // 查找根节点
        let root_node = fdt
            .all_nodes()
            .find(|node| node.as_ref().is_ok_and(|n| n.name() == "/"))
            .unwrap()
            .unwrap();

        // 测试子节点迭代器
        let children: Result<alloc::vec::Vec<_>, _> = root_node.children().collect();
        let children = children.unwrap();

        // 根节点应该有子节点
        assert!(!children.is_empty(), "根节点应该有子节点");

        // 所有子节点的 level 应该是 1
        for child in &children {
            assert_eq!(child.level(), 1, "根节点的直接子节点应该在 level 1");
        }

        // 检查是否包含一些预期的子节点
        let child_names: alloc::vec::Vec<_> = children.iter().map(|c| c.name()).collect();
        assert!(child_names.contains(&"chosen"), "应该包含 chosen 节点");
        assert!(child_names.contains(&"memory@0"), "应该包含 memory@0 节点");
    }

    #[test]
    fn test_find_child_by_name() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // 查找根节点
        let root_node = fdt
            .all_nodes()
            .find(|node| node.as_ref().is_ok_and(|n| n.name() == "/"))
            .unwrap()
            .unwrap();

        // 测试通过名称查找子节点
        let memory_node = root_node.children().find_child_by_name("memory@0").unwrap();

        assert!(memory_node.is_some(), "应该找到 memory@0 节点");
        assert_eq!(memory_node.unwrap().name(), "memory@0");

        // 测试查找不存在的节点
        let nonexistent = root_node
            .children()
            .find_child_by_name("nonexistent")
            .unwrap();
        assert!(nonexistent.is_none(), "不存在的节点应该返回 None");
    }

    #[test]
    fn test_child_iter_empty() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // 查找一个叶子节点（没有子节点的节点）
        let leaf_node = fdt
            .all_nodes()
            .find(|node| node.as_ref().is_ok_and(|n| n.name() == "chosen"))
            .unwrap()
            .unwrap();

        // 测试叶子节点的子节点迭代器
        let children: Result<alloc::vec::Vec<_>, _> = leaf_node.children().collect();
        let children = children.unwrap();

        assert!(children.is_empty(), "叶子节点不应该有子节点");
    }

    #[test]
    fn test_child_iter_multiple_levels() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // 查找 reserved-memory 节点，它应该有子节点
        let reserved_memory = fdt
            .all_nodes()
            .find(|node| node.as_ref().is_ok_and(|n| n.name() == "reserved-memory"))
            .unwrap()
            .unwrap();

        // 测试子节点迭代器
        let children: Result<alloc::vec::Vec<_>, _> = reserved_memory.children().collect();
        let children = children.unwrap();

        // 确保子节点的 level 正确
        for child in &children {
            assert_eq!(
                child.level(),
                reserved_memory.level() + 1,
                "子节点的 level 应该比父节点高 1"
            );
        }
    }
}
