use crate::{data::Raw, Fdt};

#[derive(Clone)]
pub struct Node<'a> {
    pub(crate) fdt: Fdt<'a>,
    pub(crate) name: &'a str,
    pub(crate) level: usize,
    pub(crate) pos: usize,
}

impl<'a> Node<'a> {
    pub(crate) fn new(fdt: &Fdt<'a>, name: &'a str, level: usize, pos: usize) -> Self {
        Node {
            fdt: fdt.clone(),
            name,
            level,
            pos,
        }
    }

    fn raw(&self) -> Raw<'a> {
        self.fdt.raw.begin_at(self.pos).unwrap()
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
    pub fn compatible(&self) -> Option<impl Iterator<Item = &'a str>> {
        // This is a placeholder - would need to parse properties
        None::<core::iter::Empty<&str>>
    }

    /// Get register values for this node (placeholder implementation)
    pub fn reg(&self) -> Option<impl Iterator<Item = u64>> {
        // This is a placeholder - would need to parse properties
        None::<core::iter::Empty<u64>>
    }

    pub fn kind(&self) -> NodeKind {
        NodeKind::General
    }

    /// 检查这个节点是否是根节点
    pub fn is_root(&self) -> bool {
        self.level == 0
    }

    /// 检查节点名称是否匹配指定的模式
    pub fn name_matches(&self, pattern: &str) -> bool {
        self.name == pattern
    }

    /// 检查节点名称是否以指定前缀开始
    pub fn name_starts_with(&self, prefix: &str) -> bool {
        self.name.starts_with(prefix)
    }

    /// 获取节点的完整路径信息（仅限调试用途）
    pub fn debug_info(&self) -> NodeDebugInfo<'a> {
        NodeDebugInfo {
            name: self.name,
            level: self.level,
            pos: self.pos,
        }
    }

    /// 遍历这个节点的所有子节点
    pub fn walk_children<F>(&self, callback: F) -> Result<(), crate::FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, crate::FdtError>,
    {
        self.fdt.walk_child_nodes(self.name, self.level, callback)
    }
}

/// 节点调试信息
#[derive(Debug)]
pub struct NodeDebugInfo<'a> {
    pub name: &'a str,
    pub level: usize,
    pub pos: usize,
}

#[derive(Debug)]
pub enum NodeKind {
    General,
}
