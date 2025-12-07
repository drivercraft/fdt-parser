use core::fmt::Debug;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use fdt_raw::data::StrIter;

use crate::{Phandle, Property, RangesEntry, Status, node::gerneric::NodeRefGen};

mod clock;
mod gerneric;
mod iter;
mod pci;

pub use clock::*;
pub use iter::*;
pub use pci::*;

/// 节点类型枚举，用于模式匹配
#[derive(Clone, Debug)]
pub enum NodeKind<'a> {
    Clock(NodeRefClock<'a>),
    Pci(NodeRefPci<'a>),
    Generic(NodeRefGen<'a>),
}

#[derive(Clone)]
pub struct Node {
    pub name: String,
    /// 属性列表（保持原始顺序）
    pub(crate) properties: Vec<Property>,
    /// 属性名到索引的映射（用于快速查找）
    pub(crate) prop_cache: BTreeMap<String, usize>,
    pub(crate) children: Vec<Node>,
    pub(crate) name_cache: BTreeMap<String, usize>,
}

impl Node {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            properties: Vec::new(),
            prop_cache: BTreeMap::new(),
            children: Vec::new(),
            name_cache: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn properties(&self) -> impl Iterator<Item = &Property> {
        self.properties.iter()
    }

    pub fn children(&self) -> impl Iterator<Item = &Node> {
        self.children.iter()
    }

    pub fn children_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.children.iter_mut()
    }

    pub fn add_child(&mut self, child: Node) {
        let index = self.children.len();
        self.name_cache.insert(child.name.clone(), index);
        self.children.push(child);
    }

    pub fn get_child(&self, name: &str) -> Option<&Node> {
        if let Some(&index) = self.name_cache.get(name) {
            self.children.get(index)
        } else {
            None
        }
    }

    pub fn get_child_mut(&mut self, name: &str) -> Option<&mut Node> {
        if let Some(&index) = self.name_cache.get(name) {
            self.children.get_mut(index)
        } else {
            None
        }
    }

    pub fn remove_child(&mut self, name: &str) -> Option<Node> {
        if let Some(&index) = self.name_cache.get(name) {
            self.name_cache.remove(name);
            Some(self.children.remove(index))
        } else {
            None
        }
    }

    pub fn set_property(&mut self, prop: Property) {
        let name = prop.name.clone();
        if let Some(&idx) = self.prop_cache.get(&name) {
            // 更新已存在的属性
            self.properties[idx] = prop;
        } else {
            // 添加新属性
            let idx = self.properties.len();
            self.prop_cache.insert(name, idx);
            self.properties.push(prop);
        }
    }

    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.prop_cache.get(name).map(|&idx| &self.properties[idx])
    }

    pub fn get_property_mut(&mut self, name: &str) -> Option<&mut Property> {
        self.prop_cache
            .get(name)
            .map(|&idx| &mut self.properties[idx])
    }

    pub fn remove_property(&mut self, name: &str) -> Option<Property> {
        if let Some(&idx) = self.prop_cache.get(name) {
            self.prop_cache.remove(name);
            // 重建索引（移除元素后需要更新后续索引）
            let prop = self.properties.remove(idx);
            for (_, v) in self.prop_cache.iter_mut() {
                if *v > idx {
                    *v -= 1;
                }
            }
            Some(prop)
        } else {
            None
        }
    }

    pub fn address_cells(&self) -> Option<u32> {
        self.get_property("#address-cells")
            .and_then(|prop| prop.get_u32())
    }

    pub fn size_cells(&self) -> Option<u32> {
        self.get_property("#size-cells")
            .and_then(|prop| prop.get_u32())
    }

    pub fn phandle(&self) -> Option<Phandle> {
        self.get_property("phandle")
            .and_then(|prop| prop.get_u32())
            .map(Phandle::from)
    }

    pub fn interrupt_parent(&self) -> Option<Phandle> {
        self.get_property("interrupt-parent")
            .and_then(|prop| prop.get_u32())
            .map(Phandle::from)
    }

    pub fn status(&self) -> Option<Status> {
        let prop = self.get_property("status")?;
        let s = prop.as_str()?;
        match s {
            "okay" => Some(Status::Okay),
            "disabled" => Some(Status::Disabled),
            _ => None,
        }
    }

    pub fn ranges(&self, parent_address_cells: u32) -> Option<Vec<RangesEntry>> {
        let prop = self.get_property("ranges")?;
        let mut entries = Vec::new();
        let mut reader = prop.as_reader();

        // 当前节点的 #address-cells 用于子节点地址
        let child_address_cells = self.address_cells().unwrap_or(2) as usize;
        // 父节点的 #address-cells 用于父总线地址
        let parent_addr_cells = parent_address_cells as usize;
        // 当前节点的 #size-cells
        let size_cells = self.size_cells().unwrap_or(1) as usize;

        while let (Some(child_addr), Some(parent_addr), Some(size)) = (
            reader.read_cells(child_address_cells),
            reader.read_cells(parent_addr_cells),
            reader.read_cells(size_cells),
        ) {
            entries.push(RangesEntry {
                child_bus_address: child_addr,
                parent_bus_address: parent_addr,
                length: size,
            });
        }

        Some(entries)
    }

    pub fn compatible(&self) -> Option<StrIter<'_>> {
        let prop = self.get_property("compatible")?;
        Some(prop.as_str_iter())
    }

    pub fn compatibles(&self) -> impl Iterator<Item = &str> {
        self.get_property("compatible")
            .map(|prop| prop.as_str_iter())
            .into_iter()
            .flatten()
    }

    pub fn device_type(&self) -> Option<&str> {
        let prop = self.get_property("device_type")?;
        prop.as_str()
    }

    /// 通过精确路径删除子节点及其子树
    /// 只支持精确路径匹配，不支持模糊匹配
    ///
    /// # 参数
    /// - `path`: 删除路径，格式如 "soc/gpio@1000" 或 "/soc/gpio@1000"
    ///
    /// # 返回值
    /// `Ok(Option<Node>)`: 如果找到并删除了节点，返回被删除的节点；如果路径不存在，返回 None
    /// `Err(FdtError)`: 如果路径格式无效
    ///
    /// # 示例
    /// ```rust
    /// # use fdt_edit::{Node, NodeOp};
    /// let mut root = Node::root();
    /// // 添加测试节点
    /// let mut soc = Node::new_raw("soc");
    /// soc.add_child(Node::new_raw("gpio@1000"));
    /// root.add_child(soc);
    ///
    /// // 精确删除节点
    /// let removed = root.remove_by_path("soc/gpio@1000")?;
    /// assert!(removed.is_some());
    /// # Ok::<(), fdt_raw::FdtError>(())
    /// ```
    pub fn remove_by_path(&mut self, path: &str) -> Result<Option<Node>, fdt_raw::FdtError> {
        let normalized_path = path.trim_start_matches('/');
        if normalized_path.is_empty() {
            return Err(fdt_raw::FdtError::InvalidInput);
        }

        let parts: Vec<&str> = normalized_path.split('/').collect();
        if parts.is_empty() {
            return Err(fdt_raw::FdtError::InvalidInput);
        }

        if parts.len() == 1 {
            // 删除直接子节点（精确匹配）
            let child_name = parts[0];
            Ok(self.remove_child_exact(child_name))
        } else {
            // 需要递归到父节点进行删除
            self.remove_child_recursive(&parts, 0)
        }
    }

    /// 递归删除子节点的实现
    /// 找到要删除节点的父节点，然后从父节点中删除目标子节点
    fn remove_child_recursive(
        &mut self,
        parts: &[&str],
        index: usize,
    ) -> Result<Option<Node>, fdt_raw::FdtError> {
        if index >= parts.len() - 1 {
            // 已经到达要删除节点的父级
            let child_name_to_remove = parts[index];
            Ok(self.remove_child_exact(child_name_to_remove))
        } else {
            // 继续向下递归
            let current_part = parts[index];

            // 中间级别只支持精确匹配（使用缓存）
            if let Some(&child_index) = self.name_cache.get(current_part) {
                self.children[child_index].remove_child_recursive(parts, index + 1)
            } else {
                // 路径不存在
                Ok(None)
            }
        }
    }

    /// 精确删除子节点，不支持部分匹配
    fn remove_child_exact(&mut self, name: &str) -> Option<Node> {
        if let Some(&index) = self.name_cache.get(name) {
            let child = self.children.remove(index);
            self.name_cache.remove(name);
            Some(child)
        } else {
            None
        }
    }
}

impl From<&fdt_raw::Node<'_>> for Node {
    fn from(raw: &fdt_raw::Node<'_>) -> Self {
        let mut new_node = Node::new(raw.name());
        // 复制属性
        for raw_prop in raw.properties() {
            let prop = Property::from(&raw_prop);
            new_node.set_property(prop);
        }
        new_node
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Node")
            .field("name", &self.name)
            // .field("properties", &self.properties)
            // .field("children_count", &self.children.len())
            .finish()
    }
}
