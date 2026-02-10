use core::fmt::Display;

use alloc::vec::Vec;
use fdt_raw::NodeBase;

use crate::{NodeGeneric, NodeIterMeta, NodeKind, NodeRef, NodeRefMut};

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
                    address_cells: 2, // Default value, can be overridden by root node properties
                    size_cells: 1,    // Default value, can be overridden by root node properties
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

                    // Update meta information based on the current node's properties
                    let child_meta = NodeIterMeta {
                        level: top.meta.level + 1,
                        address_cells: top.meta.address_cells, // This should be updated based on the child's properties if it has #address-cells
                        size_cells: top.meta.size_cells, // This should be updated based on the child's properties if it has #size-cells
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
