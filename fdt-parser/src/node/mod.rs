use crate::{
    data::{Buffer, Raw},
    Fdt,
};

#[derive(Clone)]
pub struct Node<'a> {
    name: &'a str,
    pub(crate) fdt: Fdt<'a>,
    pub level: usize,
    pub(crate) raw: Raw<'a>,
    pub(crate) parent_name: Option<&'a str>,
}

impl<'a> Node<'a> {
    pub(crate) fn new(
        name: &'a str,
        fdt: Fdt<'a>,
        buffer: &Buffer<'a>,
        level: usize,
        parent_name: Option<&'a str>,
    ) -> Self {
        let name = if name.is_empty() { "/" } else { name };
        Node {
            name,
            fdt,
            level,
            parent_name,
            raw: buffer.raw(),
        }
    }

    pub fn parent_name(&self) -> Option<&'a str> {
        self.parent_name
    }

    pub fn parent(&self) -> Option<Node<'a>> {
        let parent_name = self.parent_name?;
        self.fdt.all_nodes().find(|n| n.name() == parent_name)
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
    pub fn compatible(&self) -> Option<impl Iterator<Item = &'a str>> {
        // This is a placeholder - would need to parse properties
        None::<core::iter::Empty<&str>>
    }

    /// Get register values for this node (placeholder implementation)
    pub fn reg(&self) -> Option<impl Iterator<Item = u64>> {
        // This is a placeholder - would need to parse properties
        None::<core::iter::Empty<u64>>
    }

    pub fn to_kind(self) -> NodeKind<'a> {
        NodeKind::General(self)
    }

    /// 检查这个节点是否是根节点
    pub fn is_root(&self) -> bool {
        self.level == 0
    }

    /// 检查节点名称是否匹配指定的模式
    pub fn name_matches(&self, pattern: &str) -> bool {
        self.name().eq(pattern)
    }

    /// 检查节点名称是否以指定前缀开始
    pub fn name_starts_with(&self, prefix: &str) -> bool {
        self.name().starts_with(prefix)
    }

    /// 获取节点的完整路径信息（仅限调试用途）
    pub fn debug_info(&self) -> NodeDebugInfo<'a> {
        NodeDebugInfo {
            name: self.name(),
            level: self.level,
            pos: self.raw.pos,
        }
    }
}

/// 节点调试信息
#[derive(Debug)]
pub struct NodeDebugInfo<'a> {
    pub name: &'a str,
    pub level: usize,
    pub pos: usize,
}

impl core::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Node").field("name", &self.name()).finish()
    }
}

#[derive(Debug)]
pub enum NodeKind<'a> {
    General(Node<'a>),
}
