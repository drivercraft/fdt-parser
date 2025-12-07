use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice::Iter,
};

use alloc::vec::Vec;

use crate::{
    Context, Node,
    node::gerneric::{NodeMutGen, NodeRefGen},
};

#[derive(Clone, Debug)]
pub enum NodeRef<'a> {
    Gerneric(NodeRefGen<'a>),
}

impl<'a> NodeRef<'a> {
    pub fn new(node: &'a Node, ctx: Context<'a>) -> Self {
        Self::Gerneric(NodeRefGen { node, ctx })
    }
}

impl<'a> Deref for NodeRef<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            NodeRef::Gerneric(n) => n,
        }
    }
}

#[derive(Debug)]
pub enum NodeMut<'a> {
    Gerneric(NodeMutGen<'a>),
}

impl<'a> NodeMut<'a> {
    pub fn new(node: &'a mut Node, ctx: Context<'a>) -> Self {
        Self::Gerneric(NodeMutGen { node, ctx })
    }
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
    ctx: Context<'a>,
    node: Option<&'a Node>,
    stack: Vec<Iter<'a, Node>>,
}

impl<'a> NodeIter<'a> {
    pub fn new(root: &'a Node) -> Self {
        let ctx = Context::new();

        Self {
            ctx,
            node: Some(root),
            stack: vec![],
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node.take() {
            // 返回当前节点，并将其子节点压入栈中
            let ctx = self.ctx.clone();
            self.ctx.push(n);
            self.stack.push(n.children.iter());
            return Some(NodeRef::new(n, ctx));
        }

        let iter = self.stack.last_mut()?;

        if let Some(child) = iter.next() {
            // 返回子节点，并将其子节点压入栈中
            let ctx = self.ctx.clone();
            self.ctx.push(child);
            self.stack.push(child.children.iter());
            return Some(NodeRef::new(child, ctx));
        }

        // 当前迭代器耗尽，弹出栈顶
        self.stack.pop();
        self.ctx.parents.pop();
        self.next()
    }
}

pub struct NodeIterMut<'a> {
    ctx: Context<'a>,
    node: Option<NonNull<Node>>,
    stack: Vec<RawChildIter>,
    _marker: core::marker::PhantomData<&'a mut Node>,
}

/// 原始指针子节点迭代器
struct RawChildIter {
    ptr: *mut Node,
    end: *mut Node,
}

impl RawChildIter {
    fn new(children: &mut Vec<Node>) -> Self {
        let ptr = children.as_mut_ptr();
        let end = unsafe { ptr.add(children.len()) };
        Self { ptr, end }
    }

    fn next(&mut self) -> Option<NonNull<Node>> {
        if self.ptr < self.end {
            let current = self.ptr;
            self.ptr = unsafe { self.ptr.add(1) };
            NonNull::new(current)
        } else {
            None
        }
    }
}

impl<'a> NodeIterMut<'a> {
    pub fn new(root: &'a mut Node) -> Self {
        Self {
            ctx: Context::new(),
            node: Some(NonNull::from(root)),
            stack: vec![],
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for NodeIterMut<'a> {
    type Item = NodeMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node_ptr) = self.node.take() {
            // 返回当前节点，并将其子节点压入栈中
            let ctx = self.ctx.clone();
            unsafe {
                let node_ref = node_ptr.as_ref();
                self.ctx.push(node_ref);
                let node_mut = &mut *node_ptr.as_ptr();
                self.stack.push(RawChildIter::new(&mut node_mut.children));
                return Some(NodeMut::new(node_mut, ctx));
            }
        }

        let iter = self.stack.last_mut()?;

        if let Some(child_ptr) = iter.next() {
            // 返回子节点，并将其子节点压入栈中
            let ctx = self.ctx.clone();
            unsafe {
                let child_ref = child_ptr.as_ref();
                self.ctx.push(child_ref);
                let child_mut = &mut *child_ptr.as_ptr();
                self.stack.push(RawChildIter::new(&mut child_mut.children));
                return Some(NodeMut::new(child_mut, ctx));
            }
        }

        // 当前迭代器耗尽，弹出栈顶
        self.stack.pop();
        self.ctx.parents.pop();
        self.next()
    }
}
