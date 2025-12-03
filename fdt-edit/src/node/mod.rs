use alloc::{string::String, vec::Vec};

use crate::{Phandle, Property, Status};

/// 可编辑的节点
#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub properties: Vec<Property>,
    pub children: Vec<Node>,
}

impl Node {
    /// 创建新节点
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: Vec::new(),
            children: Vec::new(),
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
        self.children.push(child);
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

    /// 按名称查找子节点
    pub fn find_child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    /// 按名称查找子节点（可变）
    pub fn find_child_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.children.iter_mut().find(|c| c.name == name)
    }

    /// 移除属性
    pub fn remove_property(&mut self, name: &str) -> Option<Property> {
        if let Some(pos) = self.properties.iter().position(|p| p.name() == name) {
            Some(self.properties.remove(pos))
        } else {
            None
        }
    }

    /// 移除子节点
    pub fn remove_child(&mut self, name: &str) -> Option<Node> {
        if let Some(pos) = self.children.iter().position(|c| c.name == name) {
            Some(self.children.remove(pos))
        } else {
            None
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
