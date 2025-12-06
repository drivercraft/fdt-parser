use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use alloc::vec::Vec;

use crate::{Context, Node, Property};

#[derive(Clone, Debug)]
pub enum NodeRef<'a> {
    Gerneric(NodeRefGen<'a>),
}

impl<'a> NodeRef<'a> {
    pub fn new(node: &'a Node, ctx: Context<'a>) -> Self {
        Self::Gerneric(NodeRefGen { node, ctx })
    }
}

#[derive(Clone)]
pub struct NodeRefGen<'a> {
    pub node: &'a Node,
    pub ctx: Context<'a>,
}

impl<'a> NodeRefGen<'a> {
    pub fn find_property(&self, name: &str) -> Option<&'a Property> {
        self.node.properties.get(name)
    }

    pub fn properties(&self) -> impl Iterator<Item = &'a Property> {
        self.node.properties.values()
    }
}

impl Deref for NodeRefGen<'_> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node
    }
}

impl<'a> NodeMutGen<'a> {}

impl<'a> Deref for NodeRef<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            NodeRef::Gerneric(n) => n,
        }
    }
}

pub struct NodeMutGen<'a> {
    pub node: &'a mut Node,
    pub ctx: Context<'a>,
}

pub enum NodeMut<'a> {
    Gerneric(NodeMutGen<'a>),
}

impl<'a> Deref for NodeMut<'a> {
    type Target = NodeMutGen<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            NodeMut::Gerneric(n) => n,
        }
    }
}

impl<'a> DerefMut for NodeMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            NodeMut::Gerneric(n) => n,
        }
    }
}

pub struct NodeIter<'a> {
    stack: Vec<(&'a Node, Context<'a>)>,
}

impl<'a> NodeIter<'a> {
    pub fn new(root: &'a Node) -> Self {
        Self {
            stack: vec![(root, Context::new())],
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (node, ctx) = self.stack.pop()?;

        // 使用栈实现前序深度优先，保持原始子节点顺序
        for child in node.children.iter().rev() {
            // 为子节点创建新的上下文，当前节点成为父节点
            let child_ctx = ctx.for_child(node);
            self.stack.push((child, child_ctx));
        }

        Some(NodeRef::new(node, ctx))
    }
}

pub struct NodeIterMut<'a> {
    stack: Vec<(&'a mut Node, Context<'a>)>,
}

impl<'a> NodeIterMut<'a> {
    pub fn new(root: &'a mut Node) -> Self {
        Self {
            stack: vec![(root, Context::new())],
        }
    }
}

impl<'a> Iterator for NodeIterMut<'a> {
    type Item = NodeMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl Debug for NodeRefGen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NodeRefGen {{ name: {} }}", self.node.name())
    }
}
