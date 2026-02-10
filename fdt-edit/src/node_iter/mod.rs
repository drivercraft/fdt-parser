use alloc::vec::Vec;

use crate::Node;

struct StackEntry<'a> {
    node: &'a Node,
    child_index: usize,
}

pub(crate) struct NodeIter<'a> {
    stack: Vec<StackEntry<'a>>,
}

impl<'a> NodeIter<'a> {
    pub fn new(root: &'a Node) -> Self {
        Self {
            stack: vec![StackEntry {
                node: root,
                child_index: 0,
            }],
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(top) = self.stack.last_mut() {
            if top.child_index < top.node.children().len() {
                let child = &top.node.children()[top.child_index];
                top.child_index += 1;
                self.stack.push(StackEntry {
                    node: child,
                    child_index: 0,
                });
                return Some(child);
            } else {
                self.stack.pop();
            }
        }
        None
    }
}

struct StackEntryMut {
    node: *mut Node,
    child_index: usize,
}

unsafe impl Send for StackEntryMut {}

pub(crate) struct NodeIterMut<'a> {
    stack: Vec<StackEntryMut>,
    _marker: core::marker::PhantomData<&'a mut Node>,
}

impl<'a> NodeIterMut<'a> {
    pub fn new(root: &'a mut Node) -> Self {
        Self {
            stack: vec![StackEntryMut {
                node: root as *mut Node,
                child_index: 0,
            }],
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for NodeIterMut<'a> {
    type Item = &'a mut Node;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(top) = self.stack.last_mut() {
            unsafe {
                let node = &mut *top.node;
                if top.child_index < node.children().len() {
                    let child_ptr = &mut node.children_mut()[top.child_index] as *mut Node;
                    top.child_index += 1;
                    self.stack.push(StackEntryMut {
                        node: child_ptr,
                        child_index: 0,
                    });
                    return Some(&mut *child_ptr);
                } else {
                    self.stack.pop();
                }
            }
        }
        None
    }
}
