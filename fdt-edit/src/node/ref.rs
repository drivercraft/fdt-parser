use core::ops::{Deref, DerefMut};

use alloc::vec::Vec;

use super::Node;
use crate::{
    FdtContext, NodeOp,
    prop::{PropertyKind, RegFixed},
};

/// 带有遍历上下文的只读节点引用
#[derive(Clone, Debug)]
pub struct NodeRef<'a> {
    pub node: &'a Node,
    pub ctx: FdtContext<'a>,
}

/// 带有遍历上下文的可变节点引用
#[derive(Debug)]
pub struct NodeMut<'a> {
    pub node: &'a mut Node,
    pub ctx: FdtContext<'a>,
}

impl<'a> Deref for NodeRef<'a> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node
    }
}

impl AsRef<Node> for NodeRef<'_> {
    fn as_ref(&self) -> &Node {
        self.node
    }
}

impl<'a> Deref for NodeMut<'a> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node
    }
}

impl<'a> DerefMut for NodeMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.node
    }
}

impl<'a> NodeRef<'a> {
    /// 创建新的带上下文的节点引用
    pub(crate) fn new(node: &'a Node, context: FdtContext<'a>) -> Self {
        Self { node, ctx: context }
    }

    /// 解析 reg，按 ranges 做地址转换，返回 CPU 视角地址
    pub fn reg(&self) -> Option<Vec<RegFixed>> {
        reg_impl(self.node, &self.ctx)
    }
}

impl<'a> NodeMut<'a> {
    /// 解析 reg，按 ranges 做地址转换，返回 CPU 视角地址
    pub fn reg(&self) -> Option<Vec<RegFixed>> {
        reg_impl(self.node, &self.ctx)
    }
}

fn reg_impl(node: &Node, ctx: &FdtContext) -> Option<Vec<RegFixed>> {
    let prop = node.find_property("reg")?;
    let PropertyKind::Reg(entries) = &prop.kind else {
        return None;
    };

    // 从上下文获取当前 ranges
    let ranges = ctx.current_ranges();
    let mut out = Vec::with_capacity(entries.len());

    for entry in entries {
        let child_bus = entry.address;
        let mut cpu_addr = child_bus;

        if let Some(ref ranges) = ranges {
            for r in ranges {
                if child_bus >= r.child_bus_address && child_bus < r.child_bus_address + r.length {
                    cpu_addr = child_bus - r.child_bus_address + r.parent_bus_address;
                    break;
                }
            }
        }

        out.push(RegFixed {
            address: cpu_addr,
            child_bus_address: child_bus,
            size: entry.size,
        });
    }

    Some(out)
}
