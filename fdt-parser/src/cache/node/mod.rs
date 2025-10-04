use core::{fmt::Debug, ops::Deref};

use super::Fdt;
use crate::{
    base, data::Raw, property::PropIter, FdtError, FdtRangeSilce, FdtReg, Phandle, Property,
};

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

    pub fn full_path(&self) -> &str {
        &self.meta.full_path
    }

    pub fn parent(&self) -> Option<Node> {
        let parent_path = self.meta.parent.as_ref()?.path.as_str();
        let parent_meta = self.fdt.inner.get_node_by_path(parent_path)?;
        Some(Node::new(&self.fdt, &parent_meta))
    }

    pub fn properties<'a>(&'a self) -> Vec<Property<'a>> {
        let reader = self.raw().buffer();
        PropIter::new(self.fdt.fdt_base(), reader)
            .flatten()
            .collect()
    }

    pub fn find_property<'a>(&'a self, name: impl AsRef<str>) -> Option<Property<'a>> {
        self.properties()
            .into_iter()
            .find(|prop| prop.name == name.as_ref())
    }

    /// Get compatible strings for this node (placeholder implementation)
    pub fn compatibles(&self) -> Vec<String> {
        self.find_property("compatible")
            .map(|p| p.str_list().map(|s| s.into()).collect())
            .unwrap_or_default()
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
    full_path: String,
    pos: usize,
    pub level: usize,
    interrupt_parent: Option<Phandle>,
    parent: Option<ParentInfo>,
}

impl NodeMeta {
    pub fn new(node: &base::Node<'_>, full_path: String, parent: Option<&NodeMeta>) -> Self {
        NodeMeta {
            full_path,
            name: node.name().into(),
            pos: node.raw.pos(),
            level: node.level(),
            interrupt_parent: node.get_interrupt_parent_phandle(),
            parent: node.parent.as_ref().map(|p| ParentInfo {
                name: p.name.into(),
                path: parent.map(|n| n.full_path.clone()).unwrap_or_default(),
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
    path: String,
    level: usize,
    pos: usize,
    parent_address_cell: Option<u8>,
    pub parent_size_cell: Option<u8>,
    pub parent_name: Option<String>,
}
