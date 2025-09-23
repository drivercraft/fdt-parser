use core::{iter, ops::Deref};

use crate::{FdtError, Node};

pub struct InterruptController<'a> {
    node: Node<'a>,
}

impl<'a> InterruptController<'a> {
    pub(crate) fn new(node: Node<'a>) -> Self {
        InterruptController { node }
    }

    pub fn name(&self) -> &'a str {
        self.node.name()
    }

    pub fn interrupt_cells(&self) -> Result<u8, FdtError> {
        let prop = self
            .node
            .find_property("#interrupt-cells")?
            .ok_or(FdtError::PropertyNotFound("#interrupt-cells"))?;
        let val = prop.u32()?;
        Ok(val as u8)
    }
}

impl core::fmt::Debug for InterruptController<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut st = f.debug_struct("Memory");
        st.field("name", &self.name());

        st.finish()
    }
}

impl<'a> Deref for InterruptController<'a> {
    type Target = Node<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
