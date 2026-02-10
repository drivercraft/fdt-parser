use alloc::{vec, vec::Vec};

use crate::{NodeIterMeta, NodeRef, NodeRefMut};

use super::Node;

struct StackEntry {
    node: *mut Node,
    child_index: usize,
    meta: NodeIterMeta,
}

unsafe impl Send for StackEntry {}

pub(crate) struct NodeIter {
    stack: Vec<StackEntry>,
}

impl NodeIter {
    pub fn new(root: &Node) -> Self {
        Self {
            stack: vec![StackEntry {
                node: root as *const Node as usize as *mut Node,
                child_index: 0,
                meta: NodeIterMeta {
                    level: 0,
                    parent_path: vec![],
                },
            }],
        }
    }

    fn next_raw(&mut self) -> Option<(*mut Node, NodeIterMeta)> {
        while let Some(top) = self.stack.last_mut() {
            unsafe {
                let node = &*top.node;

                if top.child_index < node.children().len() {
                    let child = &node.children()[top.child_index];
                    let child_ptr = child as *const Node as usize as *mut Node;
                    top.child_index += 1;

                    // Build the parent path for the child node
                    let mut parent_path = top.meta.parent_path.clone();
                    parent_path.push(node.name().into());

                    let child_meta = NodeIterMeta {
                        level: top.meta.level + 1,
                        parent_path,
                    };

                    self.stack.push(StackEntry {
                        node: child_ptr,
                        child_index: 0,
                        meta: child_meta.clone(),
                    });

                    return Some((child_ptr, child_meta));
                } else {
                    self.stack.pop();
                }
            }
        }
        None
    }
}

pub(crate) struct NodeRefIter<'a> {
    iter: NodeIter,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> NodeRefIter<'a> {
    pub fn new(root: &Node) -> Self {
        Self {
            iter: NodeIter::new(root),
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for NodeRefIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next_raw()
            .map(|(node_ptr, meta)| NodeRef::new(node_ptr, meta))
    }
}

pub(crate) struct NodeRefIterMut<'a> {
    iter: NodeIter,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> NodeRefIterMut<'a> {
    pub fn new(root: &Node) -> Self {
        Self {
            iter: NodeIter::new(root),
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for NodeRefIterMut<'a> {
    type Item = NodeRefMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next_raw()
            .map(|(node_ptr, meta)| NodeRefMut::new(node_ptr, meta))
    }
}
