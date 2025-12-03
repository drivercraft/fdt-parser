use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

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
        let matching_key = self
            .children
            .keys()
            .find(|child_name| {
                let child_base = child_name.split('@').next().unwrap_or(child_name);
                child_base == name_base
            })
            .cloned();

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

    /// 查找子节点（支持智能匹配，等同于 remove_child 的查找逻辑）
    pub fn find_child(&self, name: &str) -> Option<&Node> {
        // 首先尝试精确匹配
        if let Some(child) = self.children.get(name) {
            return Some(child);
        }

        // 如果精确匹配失败，尝试部分匹配（忽略 @unit-address）
        let name_base = name.split('@').next().unwrap_or(name);

        // 找到匹配的节点
        for child in self.children.values() {
            let child_base = child.name.split('@').next().unwrap_or(&child.name);
            if child_base == name_base {
                return Some(child);
            }
        }

        None
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

    /// 根据路径查找节点
    /// 路径格式: "/node1@addr1/node2@addr2" 或 "node1@addr1/node2"
    pub fn get_by_path(&self, path: &str) -> Option<&Node> {
        // 标准化路径：去掉开头的斜杠，按斜杠分割
        let normalized_path = path.trim_start_matches('/');
        if normalized_path.is_empty() {
            return Some(self); // 空路径或根路径返回当前节点
        }

        let parts: Vec<&str> = normalized_path.split('/').collect();
        self.get_by_parts(&parts, 0)
    }

    /// 递归查找实现（不可变引用）
    fn get_by_parts(&self, parts: &[&str], index: usize) -> Option<&Node> {
        if index >= parts.len() {
            return Some(self);
        }

        let part = parts[index];

        // 查找子节点
        let child = self.children.get(part)?;

        // 在子节点中查找匹配的节点
        child.get_by_parts(parts, index + 1)
    }

    /// 根据路径查找节点（可变版本）
    /// 路径格式: "/node1@addr1/node2@addr2" 或 "node1@addr1/node2"
    pub fn get_by_path_mut(&mut self, path: &str) -> Option<&mut Node> {
        // 标准化路径：去掉开头的斜杠，按斜杠分割
        let normalized_path = path.trim_start_matches('/');
        if normalized_path.is_empty() {
            return Some(self); // 空路径或根路径返回当前节点
        }

        let parts: Vec<&str> = normalized_path.split('/').collect();
        self.get_by_parts_mut(&parts, 0)
    }

    /// 递归查找实现（可变引用）
    fn get_by_parts_mut(&mut self, parts: &[&str], index: usize) -> Option<&mut Node> {
        if index >= parts.len() {
            return Some(self);
        }

        let part = parts[index];
        // 获取可变引用并继续递归
        let child = self.children.get_mut(part)?;
        child.get_by_parts_mut(parts, index + 1)
    }

    /// 根据路径查找所有匹配的节点
    /// 支持智能匹配，返回所有找到的节点及其完整路径
    ///
    /// # 匹配规则
    /// - **中间级别**：只支持精确匹配
    /// - **最后级别**：支持精确匹配和前缀匹配
    /// - **前缀匹配**：在最后一级，节点名以指定前缀开头（忽略 @unit-address）
    ///
    /// # 参数
    /// - `path`: 查找路径，支持前缀匹配
    ///
    /// # 返回值
    /// 返回 Vec<(&Node, String)>，包含所有匹配的节点及其完整路径
    ///
    /// # 示例
    /// ```rust
    /// # use fdt_edit::Node;
    /// let mut node = Node::root();
    /// let nodes = node.find_all("gpio");      // 查找 gpio 或 gpio@xxx 等节点
    /// let nodes = node.find_all("soc/uart");   // 查找 soc/uart 或 soc/uart@1000 等节点
    /// ```
    pub fn find_all(&self, path: &str) -> Vec<(&Node, String)> {
        let normalized_path = path.trim_start_matches('/');
        if normalized_path.is_empty() {
            // 空路径返回当前节点（如果是空路径）或所有直接子节点
            if normalized_path.is_empty() {
                vec![(self, "/".to_string())]
            } else {
                let mut results = Vec::new();
                for (child_name, child) in &self.children {
                    results.push((child, format!("/{}", child_name)));
                }
                results
            }
        } else {
            let parts: Vec<&str> = normalized_path.split('/').collect();
            self.find_all_by_parts(&parts, 0, "/")
        }
    }

    /// 递归查找所有匹配节点的实现
    fn find_all_by_parts(
        &self,
        parts: &[&str],
        index: usize,
        current_path: &str,
    ) -> Vec<(&Node, String)> {
        if index >= parts.len() {
            return vec![(self, current_path.to_string())];
        }

        let part = parts[index];
        let is_last_level = index == parts.len() - 1;
        let mut results = Vec::new();

        // 普通匹配：支持精确匹配和最后一级的前缀匹配
        let matching_children = if is_last_level {
            // 最后一级：支持精确匹配和前缀匹配
            self.find_children_with_prefix(part)
        } else {
            let mut matches = Vec::new();
            // 中间级别：只支持精确匹配
            if let Some(child) = self.children.get(part) {
                matches.push((part.to_string(), child));
            }
            matches
        };

        for (child_name, child) in matching_children {
            let child_path = format!("{}{}/", current_path, child_name);

            if is_last_level {
                // 最后一级：添加匹配的子节点
                results.push((child, format!("{}{}", current_path, child_name)));
            } else {
                // 继续递归
                results.extend(child.find_all_by_parts(parts, index + 1, &child_path));
            }
        }

        results
    }

    /// 支持前缀匹配的子节点查找（最后一级使用）
    fn find_children_with_prefix(&self, prefix: &str) -> Vec<(String, &Node)> {
        let mut matches = Vec::new();

        // 找到所有匹配的键名并返回
        for (child_name, child_value) in &self.children {
            if child_name.starts_with(prefix) {
                matches.push((child_name.to_string(), child_value));
            }
        }

        matches
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
    /// # use fdt_edit::Node;
    /// let mut node = Node::root();
    /// // 精确删除节点
    /// let removed = node.remove_by_path("soc/gpio@1000")?;
    ///
    /// // 精确删除嵌套节点
    /// let removed = node.remove_by_path("soc/i2c@0/eeprom@50")?;
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
            // 删除直接子节点
            let child_name = parts[0];
            Ok(self.children.remove(child_name))
        } else {
            // 需要递归到父节点进行删除
            self.remove_child_recursive(&parts, 0)
        }
    }

    /// 递归删除子节点的实现
    /// 找到要删除节点的父节点，然后从父节点中删除目标子节点
    fn remove_child_recursive(&mut self, parts: &[&str], index: usize) -> Result<Option<Node>, fdt_raw::FdtError> {
        if index >= parts.len() - 1 {
            // 已经到达要删除节点的父级
            let child_name_to_remove = parts[index];
            Ok(self.children.remove(child_name_to_remove))
        } else {
            // 继续向下递归
            let current_part = parts[index];

            // 中间级别只支持精确匹配
            if let Some(child) = self.children.get_mut(current_part) {
                child.remove_child_recursive(parts, index + 1)
            } else {
                // 路径不存在
                Ok(None)
            }
        }
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
