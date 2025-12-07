use alloc::string::String;
use core::{fmt::Debug, ops::Deref};

use crate::{Context, Node, Property};

#[derive(Clone)]
pub struct NodeRefGen<'a> {
    pub node: &'a Node,
    pub ctx: Context<'a>,
}

impl<'a> NodeRefGen<'a> {
    pub fn find_property(&self, name: &str) -> Option<&'a Property> {
        self.node.get_property(name)
    }

    pub fn properties(&self) -> impl Iterator<Item = &'a Property> {
        self.node.properties.iter()
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

impl Deref for NodeRefGen<'_> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node
    }
}

pub struct NodeMutGen<'a> {
    pub node: &'a mut Node,
    pub ctx: Context<'a>,
}

impl<'a> NodeMutGen<'a> {
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

impl Debug for NodeRefGen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NodeRefGen {{ name: {} }}", self.node.name())
    }
}

impl Debug for NodeMutGen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NodeMutGen {{ name: {} }}", self.node.name())
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
