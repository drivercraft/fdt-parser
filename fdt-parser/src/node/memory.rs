use core::{iter, ops::Deref};

use crate::{node::RegIter, FdtError, MemoryRegion, NodeBase};

#[derive(Clone)]
pub struct Memory<'a> {
    node: NodeBase<'a>,
}

impl<'a> Memory<'a> {
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        Memory { node }
    }

    /// A memory device node is required for all devicetrees and describes the physical memory layout for the system. If a system
    /// has multiple ranges of memory, multiple memory nodes can be created, or the ranges can be specified in the reg property
    /// of a single memory node.
    pub fn regions(&self) -> impl Iterator<Item = Result<MemoryRegion, FdtError>> + 'a {
        let mut reg = self.node.reg();
        let mut has_error = false;
        let mut iter: Option<RegIter<'a>> = None;
        iter::from_fn(move || {
            if has_error {
                return None;
            }
            let it = match iter.as_mut() {
                Some(i) => i,
                None => {
                    iter = match &mut reg {
                        Ok(r) => {
                            let r = r.take()?;
                            Some(r)
                        }
                        Err(e) => {
                            has_error = true;
                            return Some(Err(e.clone()));
                        }
                    };
                    iter.as_mut()?
                }
            };
            let one = it.next()?;
            Some(Ok(MemoryRegion {
                address: one.address as usize as _,
                size: one.size.unwrap_or_default(),
            }))
        })
    }

    pub fn name(&self) -> &'a str {
        self.node.name()
    }
}

impl core::fmt::Debug for Memory<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut st = f.debug_struct("Memory");
        st.field("name", &self.name());
        for r in self.regions().flatten() {
            st.field("region", &r);
        }
        st.finish()
    }
}

impl<'a> Deref for Memory<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
