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

    /// 解析 reg，按 ranges 做地址转换，返回 CPU 视角地址
    pub fn reg(&self) -> Option<Vec<RegFixed>> {
        reg_impl(self.node, &self.context)
    }
}

impl<'a> NodeMut<'a> {
    /// 创建新的带上下文的可变节点引用
    pub(crate) fn new(node: &'a mut Node, context: FdtContext) -> Self {
        Self { node, context }
    }

    /// 解析 reg，按 ranges 做地址转换，返回 CPU 视角地址
    pub fn reg(&self) -> Option<Vec<RegFixed>> {
        reg_impl(self.node, &self.context)
    }
}

fn reg_impl(node: &Node, ctx: &FdtContext) -> Option<Vec<RegFixed>> {
    let prop = node.find_property("reg")?;
    let PropertyKind::Reg(entries) = &prop.kind else {
        return None;
    };

    let ranges_stack = ctx.ranges.last();
    let mut out = Vec::with_capacity(entries.len());

    for entry in entries {
        let child_bus = entry.address;
        let mut cpu_addr = child_bus;

        if let Some(ranges) = ranges_stack {
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
