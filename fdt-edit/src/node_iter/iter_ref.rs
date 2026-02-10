use alloc::vec::Vec;

use crate::{NodeIterMeta, NodeRef, NodeRefMut};

use super::Node;

struct StackEntry<'a> {
    node: *mut Node,
    child_index: usize,
    meta: NodeIterMeta,
    _marker: core::marker::PhantomData<&'a Node>,
}

unsafe impl<'a> Send for NodeRefIter<'a> {}

pub(crate) struct NodeRefIter<'a> {
    stack: Vec<StackEntry<'a>>,
}

impl<'a> NodeRefIter<'a> {
    pub fn new(root: &'a Node) -> Self {
        Self {
            stack: vec![StackEntry {
                node: root as *const Node as usize as *mut Node,
                child_index: 0,
                meta: NodeIterMeta {
                    level: 0,
                    address_cells: 2, // Default value, can be overridden by root node properties
                    size_cells: 1,    // Default value, can be overridden by root node properties
                },
                _marker: core::marker::PhantomData,
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
                        _marker: core::marker::PhantomData,
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

impl<'a> Iterator for NodeRefIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_raw().map(|(node_ptr, meta)| NodeRef {
            node: unsafe { &*node_ptr },
            meta,
        })
    }
}

pub(crate) struct NodeRefIterMut<'a> {
    inner: NodeRefIter<'a>,
}

impl<'a> NodeRefIterMut<'a> {
    pub fn new(root: &'a mut Node) -> Self {
        Self {
            inner: NodeRefIter::new(root),
        }
    }
}

impl<'a> Iterator for NodeRefIterMut<'a> {
    type Item = NodeRefMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_raw().map(|(node_ptr, meta)| NodeRefMut {
            node: unsafe { &mut *node_ptr },
            meta,
        })
    }
}
