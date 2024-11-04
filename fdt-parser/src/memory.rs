use core::iter;

use crate::{node::Node, MemoryRegion};

pub struct Memory<'a> {
    node: Node<'a>,
}

impl<'a> Memory<'a> {
    pub fn new(node: Node<'a>) -> Self {
        Memory { node }
    }

    /// A memory device node is required for all devicetrees and describes the physical memory layout for the system. If a system
    /// has multiple ranges of memory, multiple memory nodes can be created, or the ranges can be specified in the reg property
    /// of a single memory node.
    pub fn regions(&self) -> impl Iterator<Item = MemoryRegion> + 'a {
        let mut reg = self.node.reg();
        iter::from_fn(move || match &mut reg {
            Some(r) => {
                let reg = r.next()?;
                Some(MemoryRegion {
                    address: reg.address as usize as _,
                    size: reg.size.unwrap_or_default(),
                })
            }
            None => None,
        })
    }

    pub fn name(&self) -> &'a str {
        self.node.name()
    }
}
