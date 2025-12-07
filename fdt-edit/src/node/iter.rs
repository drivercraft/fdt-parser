use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    slice::{Iter, IterMut},
};

use alloc::{string::String, vec::Vec};

use crate::{Context, Node, Property};

#[derive(Clone, Debug)]
pub enum NodeRef<'a> {
    Gerneric(NodeRefGen<'a>),
}

impl<'a> NodeRef<'a> {
    pub fn new(node: &'a Node, ctx: Context<'a>) -> Self {
        Self::Gerneric(NodeRefGen { node, ctx })
    }

    fn op(&'a self) -> RefOp<'a> {
        RefOp {
            ctx: &self.ctx,
            node: self.node,
        }
    }

    pub fn path(&self) -> String {
        self.op().path()
    }

    pub fn path_eq(&self, path: &str) -> bool {
        self.op().ref_path_eq(path)
    }

    pub fn path_eq_fuzzy(&self, path: &str) -> bool {
        self.op().ref_path_eq_fuzzy(path)
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

impl<'a> NodeMut<'a> {
    pub fn new(node: &'a mut Node, ctx: Context<'a>) -> Self {
        Self::Gerneric(NodeMutGen { node, ctx })
    }

    fn op(&'a self) -> RefOp<'a> {
        RefOp {
            ctx: &self.ctx,
            node: self.node,
        }
    }

    pub fn path(&self) -> String {
        self.op().path()
    }

    pub fn path_eq(&self, path: &str) -> bool {
        self.op().ref_path_eq(path)
    }

    pub fn path_eq_fuzzy(&self, path: &str) -> bool {
        self.op().ref_path_eq_fuzzy(path)
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
    node: Option<&'a mut Node>,
    stack: Vec<IterMut<'a, Node>>,
}

impl<'a> NodeIterMut<'a> {
    pub fn new(root: &'a mut Node) -> Self {
        Self {
            ctx: Context::new(),
            node: Some(root),
            stack: vec![],
        }
    }
}

impl<'a> Iterator for NodeIterMut<'a> {
    type Item = NodeMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node.take() {
            // 返回当前节点，并将其子节点压入栈中
            let ctx = self.ctx.clone();
            self.ctx.push(n);
            self.stack.push(n.children.iter_mut());
            return Some(NodeMut::new(n, ctx));
        }

        let iter = self.stack.last_mut()?;

        if let Some(child) = iter.next() {
            // 返回子节点，并将其子节点压入栈中
            let ctx = self.ctx.clone();
            self.ctx.push(child);
            self.stack.push(child.children.iter_mut());
            return Some(NodeMut::new(child, ctx));
        }

        // 当前迭代器耗尽，弹出栈顶
        self.stack.pop();
        self.ctx.parents.pop();
        self.next()
    }
}

impl Debug for NodeRefGen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NodeRefGen {{ name: {} }}", self.node.name())
    }
}

struct RefOp<'a> {
    ctx: &'a Context<'a>,
    node: &'a Node,
}

impl<'a> RefOp<'a> {
    fn path(&self) -> String {
        self.ctx.current_path() + "/" + self.node.name()
    }

    fn ref_path_eq(&self, path: &str) -> bool {
        self.path() == path
    }

    fn ref_path_eq_fuzzy(&self, path: &str) -> bool {
        let mut want = path.trim_matches('/').split("/");
        let got_path = self.path();
        let mut got = got_path.trim_matches('/').split("/");
        let got_count = got.clone().count();
        let mut current = 0;

        loop {
            let w = want.next();
            let g = got.next();
            let is_last = current + 1 == got_count;

            match (w, g) {
                (Some(w), Some(g)) => {
                    if w != g && !is_last {
                        return false;
                    }

                    let name = g.split('@').next().unwrap_or(g);
                    let addr = g.split('@').nth(1);

                    let want_name = w.split('@').next().unwrap_or(w);
                    let want_addr = w.split('@').nth(1);

                    let res = match (addr, want_addr) {
                        (Some(a), Some(wa)) => name == want_name && a == wa,
                        (Some(_), None) => name == want_name,
                        (None, Some(_)) => false,
                        (None, None) => name == want_name,
                    };
                    if !res {
                        return false;
                    }
                }
                (None, _) => break,
                _ => return false,
            }
            current += 1;
        }
        true
    }
}
