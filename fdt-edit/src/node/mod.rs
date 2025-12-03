use alloc::{string::String, vec::Vec, collections::BTreeMap};

use crate::{Phandle, Property, Status};

/// 可编辑的节点
#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub properties: Vec<Property>,
    pub children: BTreeMap<String, Node>,
}

impl Node {
    /// 创建新节点
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: Vec::new(),
            children: BTreeMap::new(),
        }
    }

    /// 创建根节点
    pub fn root() -> Self {
        Self::new("")
    }

    /// 添加属性
    pub fn add_property(&mut self, prop: Property) -> &mut Self {
        self.properties.push(prop);
        self
    }

    /// 添加子节点
    pub fn add_child(&mut self, child: Node) -> &mut Self {
        let child_name = child.name.clone();
        self.children.insert(child_name, child);
        self
    }

    /// 按名称查找属性
    pub fn find_property(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name() == name)
    }

    /// 按名称查找属性（可变）
    pub fn find_property_mut(&mut self, name: &str) -> Option<&mut Property> {
        self.properties.iter_mut().find(|p| p.name() == name)
    }

    /// 按名称查找子节点，支持 node-name@unit-address 格式
    ///
    /// # 匹配规则
    /// - 精确匹配：如果名称包含 @，优先精确匹配完整名称
    /// - 部分匹配：如果精确匹配失败，尝试匹配节点名部分（忽略 @unit-address）
    pub fn find_child(&self, name: &str) -> Option<&Node> {
        // 首先尝试精确匹配
        if let Some(child) = self.children.get(name) {
            return Some(child);
        }

        // 如果精确匹配失败，尝试部分匹配（忽略 @unit-address）
        let name_base = name.split('@').next().unwrap_or(name);

        for child in self.children.values() {
            let child_base = child.name.split('@').next().unwrap_or(&child.name);
            if child_base == name_base {
                return Some(child);
            }
        }

        None
    }

    /// 按名称查找子节点（可变），支持 node-name@unit-address 格式
    pub fn find_child_mut(&mut self, name: &str) -> Option<&mut Node> {
        // 首先尝试精确匹配
        if self.children.contains_key(name) {
            return self.children.get_mut(name);
        }

        // 如果精确匹配失败，尝试部分匹配（忽略 @unit-address）
        let name_base = name.split('@').next().unwrap_or(name);

        // 找到匹配的键
        let matching_key = self.children.keys().find(|child_name| {
            let child_base = child_name.split('@').next().unwrap_or(child_name);
            child_base == name_base
        });

        if let Some(key) = matching_key {
            self.children.get_mut(key)
        } else {
            None
        }
    }

    /// 移除属性
    pub fn remove_property(&mut self, name: &str) -> Option<Property> {
        if let Some(pos) = self.properties.iter().position(|p| p.name() == name) {
            Some(self.properties.remove(pos))
        } else {
            None
        }
    }

    /// 移除子节点，支持 node-name@unit-address 格式
    ///
    /// # 匹配规则
    /// - 精确匹配：如果名称包含 @，优先精确匹配完整名称
    /// - 部分匹配：如果精确匹配失败，尝试匹配节点名部分（忽略 @unit-address）
    pub fn remove_child(&mut self, name: &str) -> Option<Node> {
        // 首先尝试精确匹配
        if let Some(child) = self.children.remove(name) {
            return Some(child);
        }

        // 如果精确匹配失败，尝试部分匹配（忽略 @unit-address）
        let name_base = name.split('@').next().unwrap_or(name);

        // 找到匹配的节点名称
        let matching_key = self.children.keys().find(|child_name| {
            let child_base = child_name.split('@').next().unwrap_or(child_name);
            child_base == name_base
        }).cloned();

        if let Some(key) = matching_key {
            self.children.remove(&key)
        } else {
            None
        }
    }

    /// 精确匹配子节点，不支持部分匹配
    pub fn find_child_exact(&self, name: &str) -> Option<&Node> {
        self.children.get(name)
    }

    /// 精确匹配子节点（可变），不支持部分匹配
    pub fn find_child_exact_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.children.get_mut(name)
    }

    /// 获取所有子节点名称（按字典序排序）
    pub fn child_names(&self) -> Vec<&String> {
        self.children.keys().collect()
    }

    /// 获取子节点数量
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// 检查是否有指定名称的子节点（支持部分匹配）
    pub fn has_child(&self, name: &str) -> bool {
        self.find_child(name).is_some()
    }

    /// 精确检查是否有指定名称的子节点
    pub fn has_child_exact(&self, name: &str) -> bool {
        self.children.contains_key(name)
    }

    /// 兼容性方法：获取子节点的 Vec 形式
    pub fn children_vec(&self) -> Vec<&Node> {
        self.children.values().collect()
    }

    /// 兼容性方法：获取子节点的可变 Vec 形式
    pub fn children_vec_mut(&mut self) -> Vec<&mut Node> {
        self.children.values_mut().map(|(_, v)| v).collect()
    }

    /// 设置或更新属性
    pub fn set_property(&mut self, prop: Property) -> &mut Self {
        let name = prop.name();
        if let Some(pos) = self.properties.iter().position(|p| p.name() == name) {
            self.properties[pos] = prop;
        } else {
            self.properties.push(prop);
        }
        self
    }

    /// 获取 #address-cells 值
    pub fn address_cells(&self) -> Option<u8> {
        self.find_property("#address-cells").and_then(|p| match p {
            Property::AddressCells(v) => Some(*v),
            _ => None,
        })
    }

    /// 获取 #size-cells 值
    pub fn size_cells(&self) -> Option<u8> {
        self.find_property("#size-cells").and_then(|p| match p {
            Property::SizeCells(v) => Some(*v),
            _ => None,
        })
    }

    /// 获取 phandle 值
    pub fn phandle(&self) -> Option<Phandle> {
        self.find_property("phandle")
            .and_then(|p| match p {
                Property::Phandle(v) => Some(*v),
                _ => None,
            })
            .or_else(|| {
                // 也检查 linux,phandle
                self.find_property("linux,phandle").and_then(|p| match p {
                    Property::LinuxPhandle(v) => Some(*v),
                    _ => None,
                })
            })
    }

    pub fn status(&self) -> Option<Status> {
        for prop in &self.properties {
            if let Property::Status(s) = prop {
                return Some(*s);
            }
        }
        None
    }
}

impl<'a> From<fdt_raw::Node<'a>> for Node {
    fn from(raw_node: fdt_raw::Node<'a>) -> Self {
        let mut node = Node::new(raw_node.name());

        // 转换属性
        for prop in raw_node.properties() {
            node.properties.push(Property::from(prop));
        }

        node
    }
}
