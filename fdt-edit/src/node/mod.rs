use core::ops::{Deref, DerefMut};

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::{Phandle, Property, Status, prop::PropertyKind};

mod chosen;
mod memory;
mod pci;
mod r#ref;
pub(crate) mod write;

pub use chosen::NodeChosen;
pub use memory::{MemoryRegion, NodeMemory};
pub use pci::*;
pub use r#ref::{NodeMut, NodeRef};

#[enum_dispatch::enum_dispatch]
#[derive(Clone, Debug)]
pub enum Node {
    Raw(RawNode),
    Pci(NodePci),
    Chosen(NodeChosen),
    Memory(NodeMemory),
}

impl Deref for Node {
    type Target = RawNode;

    fn deref(&self) -> &Self::Target {
        self.as_raw()
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_raw_mut()
    }
}

impl Node {
    /// 创建根节点
    pub fn root() -> Self {
        Self::Raw(RawNode::new(""))
    }

    fn new(raw: RawNode) -> Self {
        let name = raw.name.as_str();

        // 根据节点名称或属性判断类型
        if name == "chosen" {
            return Self::Chosen(NodeChosen(raw));
        }

        if name.starts_with("memory") {
            return Self::Memory(NodeMemory(raw));
        }

        // 检查 device_type 属性
        let mut node = Self::Raw(raw);
        if let Some(t) = node.find_property("device_type")
            && let PropertyKind::Str(dt) = &t.kind
            && dt.as_str() == "pci"
        {
            node = Self::Pci(NodePci(node.to_raw()));
        }
        node
    }

    pub fn new_raw(name: &str) -> Self {
        Self::new(RawNode::new(name))
    }

    /// 尝试转换为 Chosen 节点
    pub fn as_chosen(&self) -> Option<&NodeChosen> {
        if let Node::Chosen(c) = self {
            Some(c)
        } else {
            None
        }
    }

    /// 尝试转换为 Memory 节点
    pub fn as_memory(&self) -> Option<&NodeMemory> {
        if let Node::Memory(m) = self {
            Some(m)
        } else {
            None
        }
    }

    /// 尝试转换为 Pci 节点
    pub fn as_pci(&self) -> Option<&NodePci> {
        if let Node::Pci(p) = self {
            Some(p)
        } else {
            None
        }
    }
}

#[enum_dispatch::enum_dispatch(Node)]
pub trait NodeTrait {
    fn as_raw(&self) -> &RawNode;
    fn as_raw_mut(&mut self) -> &mut RawNode;
    fn to_raw(self) -> RawNode;
}

impl NodeTrait for RawNode {
    fn as_raw(&self) -> &RawNode {
        self
    }
    fn as_raw_mut(&mut self) -> &mut RawNode {
        self
    }
    fn to_raw(self) -> RawNode {
        self
    }
}

impl NodeOp for Node {}
impl NodeOp for RawNode {}

pub trait NodeOp: NodeTrait {
    fn name(&self) -> &str {
        &self.as_raw().name
    }

    fn children(&self) -> core::slice::Iter<'_, Node> {
        self.as_raw().children.iter()
    }

    fn children_mut(&mut self) -> core::slice::IterMut<'_, Node> {
        self.as_raw_mut().children.iter_mut()
    }

    /// 获取所有子节点名称（按原始顺序）
    fn child_names(&self) -> Vec<&str> {
        self.children().map(|child| child.name()).collect()
    }

    /// 获取子节点数量
    fn child_count(&self) -> usize {
        self.as_raw().children.len()
    }

    /// 按索引获取子节点
    fn child_at(&self, index: usize) -> Option<&Node> {
        self.as_raw().children.get(index)
    }

    /// 按索引获取子节点（可变）
    fn child_at_mut(&mut self, index: usize) -> Option<&mut Node> {
        self.as_raw_mut().children.get_mut(index)
    }

    /// 重建子节点缓存索引
    /// 当手动修改children向量时调用此方法重新建立索引
    fn rebuild_children_cache(&mut self) {
        self.as_raw_mut().children_cache.clear();
        let mut elem = vec![];

        for (index, child) in self.children().enumerate() {
            elem.push((child.name().to_string(), index));
        }

        for (name, index) in elem {
            self.as_raw_mut().children_cache.insert(name, index);
        }
    }

    /// 添加子节点
    fn add_child(&mut self, child: Node) -> &mut Self {
        let child_name = child.name().to_string();
        let index = self.child_count();
        self.as_raw_mut().children.push(child);
        self.as_raw_mut().children_cache.insert(child_name, index);
        self
    }

    /// 添加属性
    fn add_property(&mut self, prop: Property) -> &mut Self {
        self.as_raw_mut().properties.push(prop);
        self
    }

    fn properties(&self) -> core::slice::Iter<'_, Property> {
        self.as_raw().properties.iter()
    }

    fn properties_mut(&mut self) -> core::slice::IterMut<'_, Property> {
        self.as_raw_mut().properties.iter_mut()
    }

    /// 按名称查找属性
    fn find_property(&self, name: &str) -> Option<&Property> {
        self.properties().find(|p| p.name() == name)
    }

    /// 按名称查找属性（可变）
    fn find_property_mut(&mut self, name: &str) -> Option<&mut Property> {
        self.properties_mut().find(|p| p.name() == name)
    }

    /// 移除属性
    fn remove_property(&mut self, name: &str) -> Option<Property> {
        self.properties()
            .position(|p| p.name() == name)
            .map(|pos| self.as_raw_mut().properties.remove(pos))
    }

    /// 移除子节点，支持 node-name@unit-address 格式
    ///
    /// # 匹配规则
    /// - 精确匹配：如果名称包含 @，优先精确匹配完整名称
    /// - 部分匹配：如果精确匹配失败，尝试匹配节点名部分（忽略 @unit-address）
    fn remove_child(&mut self, name: &str) -> Option<Node> {
        // 首先尝试精确匹配（使用缓存）
        if let Some(&index) = self.as_raw().children_cache.get(name) {
            let child = self.as_raw_mut().children.remove(index);
            self.as_raw_mut().children_cache.remove(name);
            // 重建缓存，因为索引已经改变
            self.rebuild_children_cache();
            return Some(child);
        }

        // 如果精确匹配失败，尝试部分匹配（忽略 @unit-address）
        let name_base = name.split('@').next().unwrap_or(name);

        // 找到匹配的节点名称
        let matching_index = self.children().position(|child| {
            let child_base = child.name().split('@').next().unwrap_or(child.name());
            child_base == name_base
        });

        if let Some(index) = matching_index {
            let child = self.as_raw_mut().children.remove(index);
            // 重建缓存，因为索引已经改变
            self.rebuild_children_cache();
            Some(child)
        } else {
            None
        }
    }

    /// 精确匹配子节点，不支持部分匹配
    fn find_child_exact(&self, name: &str) -> Option<&Node> {
        if let Some(&index) = self.as_raw().children_cache.get(name) {
            self.as_raw().children.get(index)
        } else {
            None
        }
    }

    /// 查找子节点（支持智能匹配，等同于 remove_child 的查找逻辑）
    fn find_child(&self, name: &str) -> Vec<&Node> {
        let mut results = Vec::new();
        // 首先尝试精确匹配（使用缓存）
        if let Some(&index) = self.as_raw().children_cache.get(name) {
            results.push(self.as_raw().children.get(index).unwrap());

            return results;
        }

        // 如果精确匹配失败，尝试部分匹配（忽略 @unit-address）
        let name_base = name.split('@').next().unwrap_or(name);

        // 找到匹配的节点
        for child in self.children() {
            let child_base = child.name().split('@').next().unwrap_or(child.name());
            if child_base == name_base {
                results.push(child);
            }
        }

        results
    }

    /// 精确匹配子节点（可变），不支持部分匹配
    fn find_child_exact_mut(&mut self, name: &str) -> Option<&mut Node> {
        if let Some(&index) = self.as_raw().children_cache.get(name) {
            self.as_raw_mut().children.get_mut(index)
        } else {
            None
        }
    }

    /// 查找子节点（支持智能匹配，等同于 remove_child 的查找逻辑）
    fn find_child_mut(&mut self, name: &str) -> Option<&mut Node> {
        // 首先尝试精确匹配（使用缓存）
        if let Some(&index) = self.as_raw().children_cache.get(name) {
            return self.as_raw_mut().children.get_mut(index);
        }

        // 如果精确匹配失败，尝试部分匹配（忽略 @unit-address）
        let name_base = name.split('@').next().unwrap_or(name);

        // 找到匹配的节点
        for child in self.children_mut() {
            let child_base = child.name().split('@').next().unwrap_or(child.name());
            if child_base == name_base {
                return Some(child);
            }
        }

        None
    }

    /// 精确删除子节点，不支持部分匹配
    fn remove_child_exact(&mut self, name: &str) -> Option<Node> {
        if let Some(&index) = self.as_raw_mut().children_cache.get(name) {
            let child = self.as_raw_mut().children.remove(index);
            self.as_raw_mut().children_cache.remove(name);
            // 重建缓存，因为索引已经改变
            self.rebuild_children_cache();
            Some(child)
        } else {
            None
        }
    }

    /// 获取所有子节点名称（按字典序排序）
    fn child_names_sorted(&self) -> Vec<&str> {
        let mut names = self
            .children()
            .map(|child| child.name())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    /// 设置或更新属性
    fn set_property(&mut self, prop: Property) -> &mut Self {
        let name = prop.name();
        if let Some(pos) = self.properties().position(|p| p.name() == name) {
            self.as_raw_mut().properties[pos] = prop;
        } else {
            self.as_raw_mut().properties.push(prop);
        }
        self
    }

    /// 获取 #address-cells 值
    fn address_cells(&self) -> Option<u8> {
        let prop = self.find_property("#address-cells")?;
        let PropertyKind::Num(v) = &prop.kind else {
            return None;
        };
        Some(*v as _)
    }

    /// 获取 #size-cells 值
    fn size_cells(&self) -> Option<u8> {
        let prop = self.find_property("#size-cells")?;
        let PropertyKind::Num(v) = &prop.kind else {
            return None;
        };
        Some(*v as _)
    }

    /// 获取 phandle 值
    fn phandle(&self) -> Option<Phandle> {
        let prop = self.find_property("phandle")?;
        match prop.kind {
            PropertyKind::Phandle(p) => Some(p),
            _ => None,
        }
    }

    fn status(&self) -> Option<Status> {
        let prop = self.find_property("status")?;
        match &prop.kind {
            PropertyKind::Status(s) => Some(*s),
            _ => None,
        }
    }

    fn interrupt_parent(&self) -> Option<Phandle> {
        let prop = self.find_property("interrupt-parent")?;
        match prop.kind {
            PropertyKind::Phandle(p) => Some(p),
            _ => None,
        }
    }

    fn device_type(&self) -> Option<&str> {
        let prop = self.find_property("device_type")?;
        match &prop.kind {
            PropertyKind::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn compatibles(&self) -> Vec<&str> {
        let mut res = vec![];

        if let Some(prop) = self.find_property("compatible") {
            match &prop.kind {
                PropertyKind::StringList(list) => {
                    for s in list {
                        res.push(s.as_str());
                    }
                }
                PropertyKind::Str(s) => {
                    res.push(s.as_str());
                }
                _ => unreachable!(),
            }
        }

        res
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
    fn remove_by_path(&mut self, path: &str) -> Result<Option<Node>, fdt_raw::FdtError> {
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
            if let Some(&child_index) = self.as_raw().children_cache.get(current_part) {
                self.as_raw_mut().children[child_index].remove_child_recursive(parts, index + 1)
            } else {
                // 路径不存在
                Ok(None)
            }
        }
    }
}

/// 可编辑的节点
#[derive(Clone, Debug)]
pub struct RawNode {
    pub name: String,
    pub properties: Vec<Property>,
    // 子节点列表，保持原始顺序
    pub children: Vec<Node>,
    // 名称到索引的缓存映射，用于快速查找
    children_cache: BTreeMap<String, usize>,
}

impl RawNode {
    /// 创建新节点
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: Vec::new(),
            children: Vec::new(),
            children_cache: BTreeMap::new(),
        }
    }
}

impl<'a> From<fdt_raw::Node<'a>> for Node {
    fn from(raw_node: fdt_raw::Node<'a>) -> Self {
        let mut node = RawNode::new(raw_node.name());
        // 转换属性
        for prop in raw_node.properties() {
            let raw = Property::from(prop);
            node.properties.push(raw);
        }
        Self::new(node)
    }
}
