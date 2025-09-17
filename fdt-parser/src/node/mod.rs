use crate::{data::Raw, property::PropIter, Fdt, FdtError, Property};

#[derive(Clone)]
pub struct Node<'a> {
    name: &'a str,
    pub(crate) fdt: Fdt<'a>,
    pub level: usize,
    pub(crate) raw: Raw<'a>,
    pub(crate) parent: Option<ParentInfo<'a>>,
}

#[derive(Clone)]
pub(crate) struct ParentInfo<'a> {
    name: &'a str,
    level: usize,
    raw: Raw<'a>,
}

impl<'a> Node<'a> {
    pub(crate) fn new(
        name: &'a str,
        fdt: Fdt<'a>,
        raw: Raw<'a>,
        level: usize,
        parent: Option<&Node<'a>>,
    ) -> Self {
        let name = if name.is_empty() { "/" } else { name };
        Node {
            name,
            fdt,
            level,
            parent: parent.map(|p| ParentInfo {
                name: p.name(),
                level: p.level(),
                raw: p.raw(),
            }),
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

    fn parent_fast(&self) -> Option<Node<'a>> {
        self.parent.as_ref().map(|p| Node {
            name: p.name,
            fdt: self.fdt.clone(),
            level: p.level,
            raw: p.raw,
            parent: None,
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
    pub fn compatible(&self) -> Result<Option<impl Iterator<Item = &'a str> + 'a>, FdtError> {
        let prop = self.find_property("compatible")?;
        if let Some(prop) = &prop {
            return Ok(Some(prop.str_list()));
        }
        Ok(None)
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
