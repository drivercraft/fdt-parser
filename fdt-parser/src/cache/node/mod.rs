use core::{fmt::Debug, ops::Deref};

use super::Fdt;
use crate::{base, data::Raw, property::PropIter, FdtError, Phandle, Property};

mod chosen;

mod memory;

use alloc::{string::String, vec::Vec};
pub use chosen::*;
pub use memory::*;

#[derive(Debug, Clone)]
pub enum Node {
    General(NodeBase),
}

impl Node {
    pub(super) fn new(fdt: &Fdt, meta: &NodeMeta) -> Self {
        let base = NodeBase {
            fdt: fdt.clone(),
            meta: meta.clone(),
        };
        Self::General(base)
    }
}

impl Deref for Node {
    type Target = NodeBase;

    fn deref(&self) -> &Self::Target {
        match self {
            Node::General(n) => n,
        }
    }
}

#[derive(Clone)]
pub struct NodeBase {
    fdt: Fdt,
    meta: NodeMeta,
}

impl NodeBase {
    fn raw<'a>(&'a self) -> Raw<'a> {
        self.fdt.raw().begin_at(self.meta.pos)
    }

    pub fn level(&self) -> usize {
        self.meta.level
    }

    pub fn name(&self) -> &str {
        &self.meta.name
    }

    pub fn parent_name(&self) -> Option<&str> {
        self.meta
            .parent
            .as_ref()
            .and_then(|p| p.parent_name.as_deref())
    }

    pub fn properties<'a>(&'a self) -> Result<Vec<Property<'a>>, FdtError> {
        let reader = self.raw().buffer();
        PropIter::new(self.fdt.fdt_base(), reader).collect()
    }

    pub fn find_property<'a>(
        &'a self,
        name: impl AsRef<str>,
    ) -> Result<Option<Property<'a>>, FdtError> {
        for prop in self.properties()? {
            if prop.name == name.as_ref() {
                return Ok(Some(prop));
            }
        }
        Ok(None)
    }

    /// Get compatible strings for this node (placeholder implementation)
    pub fn compatibles(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(Some(p)) = self.find_property("compatible") {
            out = p.str_list().map(|s| s.into()).collect();
        }
        out
    }
}

impl Debug for NodeBase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut st = f.debug_struct("NodeBase");
        // st.field("name", &self.name());
        st.finish()
    }
}

#[derive(Clone)]
pub(super) struct NodeMeta {
    name: String,
    pos: usize,
    level: usize,
    interrupt_parent: Option<Phandle>,
    parent: Option<ParentInfo>,
}

impl NodeMeta {
    pub fn new(node: &base::Node<'_>) -> Self {
        NodeMeta {
            name: node.name().into(),
            pos: node.raw.pos(),
            level: node.level(),
            interrupt_parent: node.get_interrupt_parent_phandle(),
            parent: node.parent.as_ref().map(|p| ParentInfo {
                name: p.name.into(),
                level: p.level,
                pos: p.raw.pos(),
                parent_address_cell: p.parent_address_cell,
                parent_size_cell: p.parent_size_cell,
                parent_name: p.parent_name.map(|s| s.into()),
            }),
        }
    }
}

#[derive(Clone)]
struct ParentInfo {
    name: String,
    level: usize,
    pos: usize,
    parent_address_cell: Option<u8>,
    pub parent_size_cell: Option<u8>,
    pub parent_name: Option<String>,
}
