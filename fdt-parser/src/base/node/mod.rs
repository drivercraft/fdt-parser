use core::ops::Deref;

use super::Fdt;
use crate::{
    base::NodeIter,
    data::{Buffer, Raw, U32Iter2D},
    property::PropIter,
    FdtError, FdtRangeSilce, FdtReg, Phandle, Property, Status,
};

mod chosen;
mod interrupt_controller;
mod memory;

pub use chosen::*;
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
    // Parent's #address-cells and #size-cells (for parsing reg)
    pub address_cells: Option<u8>,
    pub size_cells: Option<u8>,
    // Parent's ranges for address translation
    pub ranges: Option<FdtRangeSilce<'a>>,
}

impl<'a> NodeBase<'a> {
    /// Create a new NodeBase with pre-calculated parent information from the stack
    pub(crate) fn new_with_parent_info(
        name: &'a str,
        fdt: Fdt<'a>,
        raw: Raw<'a>,
        level: usize,
        parent: Option<&NodeBase<'a>>,
        parent_address_cells: Option<u8>,
        parent_size_cells: Option<u8>,
        parent_ranges: Option<FdtRangeSilce<'a>>,
        interrupt_parent: Option<Phandle>,
    ) -> Self {
        let name = if name.is_empty() { "/" } else { name };
        NodeBase {
            name,
            fdt,
            level,
            parent: parent.map(|p| ParentInfo {
                name: p.name(),
                level: p.level(),
                raw: p.raw(),
                address_cells: parent_address_cells,
                size_cells: parent_size_cells,
                ranges: parent_ranges,
            }),
            interrupt_parent,
            raw,
        }
    }

    pub fn parent_name(&self) -> Option<&'a str> {
        self.parent_fast().map(|p| p.name())
    }

    pub fn parent(&self) -> Option<Node<'a>> {
        let parent_info = self.parent.as_ref()?;
        self.fdt
            .all_nodes()
            .flatten()
            .find(|node| node.name() == parent_info.name && node.level() == parent_info.level)
    }

    pub(crate) fn parent_fast(&self) -> Option<NodeBase<'a>> {
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
    pub fn compatibles(&self) -> Result<impl Iterator<Item = &'a str> + 'a, FdtError> {
        let prop = self.find_property("compatible")?;
        Ok(prop.str_list())
    }

    pub fn compatibles_flatten(&self) -> Result<impl Iterator<Item = &'a str> + 'a, FdtError> {
        self.compatibles()
    }

    pub fn reg(&self) -> Result<RegIter<'a>, FdtError> {
        let prop = self.find_property("reg")?;

        // Get parent info from ParentInfo structure
        let parent_info = self
            .parent
            .as_ref()
            .ok_or(FdtError::NodeNotFound("parent"))?;

        // reg parsing uses the immediate parent's cells
        let address_cell = parent_info.address_cells.unwrap_or(2);
        let size_cell = parent_info.size_cells.unwrap_or(1);

        // Use parent's pre-calculated ranges for address translation
        let ranges = parent_info.ranges.clone();

        Ok(RegIter {
            size_cell,
            address_cell,
            buff: prop.data.buffer(),
            ranges,
        })
    }

    fn is_interrupt_controller(&self) -> bool {
        self.find_property("#interrupt-controller").is_ok()
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

    pub fn find_property(&self, name: &str) -> Result<Property<'a>, FdtError> {
        for prop in self.properties() {
            let prop = prop?;
            if prop.name.eq(name) {
                return Ok(prop);
            }
        }
        Err(FdtError::NotFound)
    }

    pub fn phandle(&self) -> Result<Phandle, FdtError> {
        let prop = self.find_property("phandle")?;
        Ok(prop.u32()?.into())
    }

    /// Find [InterruptController] from current node or its parent
    pub fn interrupt_parent(&self) -> Result<InterruptController<'a>, FdtError> {
        // First try to get the interrupt parent phandle from the node itself
        let phandle = self.interrupt_parent.ok_or(FdtError::NotFound)?;

        // Find the node with this phandle
        let node = self.fdt.get_node_by_phandle(phandle)?;
        match node {
            Node::InterruptController(ic) => Ok(ic),
            _ => Err(FdtError::NodeNotFound("interrupt-parent")),
        }
    }

    /// Get the interrupt parent phandle for this node
    pub fn get_interrupt_parent_phandle(&self) -> Option<Phandle> {
        self.interrupt_parent
    }

    pub fn interrupts(
        &self,
    ) -> Result<impl Iterator<Item = impl Iterator<Item = u32> + 'a> + 'a, FdtError> {
        let prop = self.find_property("interrupts")?;
        let irq_parent = self.interrupt_parent()?;
        let cell_size = irq_parent.interrupt_cells()?;
        let iter = U32Iter2D::new(&prop.data, cell_size);

        Ok(iter)
    }

    pub fn clock_frequency(&self) -> Result<u32, FdtError> {
        let prop = self.find_property("clock-frequency")?;
        Ok(prop.u32()?)
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

    pub fn status(&self) -> Result<Status, FdtError> {
        let prop = self.find_property("status")?;
        let s = prop.str()?;

        if s.contains("disabled") {
            return Ok(Status::Disabled);
        }

        if s.contains("okay") {
            return Ok(Status::Okay);
        }

        Err(FdtError::NotFound)
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
    pub(crate) size_cell: u8,
    pub(crate) address_cell: u8,
    pub(crate) buff: Buffer<'a>,
    pub(crate) ranges: Option<FdtRangeSilce<'a>>,
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
    all_nodes: Option<NodeIter<'a, 16>>,
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
    pub fn find_child_by_name(self, name: &str) -> Result<Node<'a>, FdtError> {
        for child_result in self {
            let child = child_result?;
            if child.name() == name {
                return Ok(child);
            }
        }
        Err(FdtError::NotFound)
    }

    /// 查找具有特定兼容性字符串的子节点
    pub fn find_child_by_compatible(self, compatible: &str) -> Result<Node<'a>, FdtError> {
        for child_result in self {
            let child = child_result?;
            match child.compatibles() {
                Ok(mut compatibles) => {
                    if compatibles.any(|comp| comp == compatible) {
                        return Ok(child);
                    }
                }
                Err(FdtError::NotFound) => {}
                Err(e) => return Err(e),
            }
        }
        Err(FdtError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::{Fdt, FdtError};

    #[test]
    fn test_node_child_iter_basic() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // 查找根节点
        let root_node = fdt
            .find_nodes("/")
            .next()
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
            .find_nodes("/")
            .next()
            .unwrap()
            .unwrap();

        // 测试通过名称查找子节点
        let memory_node = root_node
            .children()
            .find_child_by_name("memory@0")
            .unwrap();

        assert_eq!(memory_node.name(), "memory@0");

        // 测试查找不存在的节点
        let nonexistent_err = root_node
            .children()
            .find_child_by_name("nonexistent")
            .unwrap_err();
        assert!(matches!(nonexistent_err, FdtError::NotFound));
    }

    #[test]
    fn test_child_iter_empty() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // 查找一个叶子节点（没有子节点的节点）
        let leaf_node = fdt
            .find_nodes("/chosen")
            .next()
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
