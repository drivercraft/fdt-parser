use alloc::vec::Vec;

use super::Node;
use crate::{
    FdtContext, NodeOp, Property,
    prop::{PropertyKind, RegFixed},
};

/// 带有遍历上下文的只读节点引用
#[derive(Clone, Debug)]
pub struct NodeRef<'a> {
    pub node: &'a Node,
    pub context: FdtContext,
}

/// 带有遍历上下文的可变节点引用
#[derive(Debug)]
pub struct NodeMut<'a> {
    pub node: &'a mut Node,
    pub context: FdtContext,
}

impl<'a> NodeRef<'a> {
    /// 创建新的带上下文的节点引用
    pub(crate) fn new(node: &'a Node, context: FdtContext) -> Self {
        Self { node, context }
    }
}

impl<'a> NodeMut<'a> {
    /// 创建新的带上下文的可变节点引用
    pub(crate) fn new(node: &'a mut Node, context: FdtContext) -> Self {
        Self { node, context }
    }
}

impl<'a> NodeRefOp for NodeMut<'a> {
    fn node(&self) -> &Node {
        self.node
    }
}

impl<'a> NodeRefOp for NodeRef<'a> {
    fn node(&self) -> &Node {
        self.node
    }
}

pub trait NodeRefOp {
    fn node(&self) -> &Node;

    fn reg(&self) -> Vec<RegFixed> {
        Vec::new()
    }
}
