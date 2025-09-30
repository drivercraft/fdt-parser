use core::ops::Deref;

use super::NodeBase;
use crate::FdtError;

#[derive(Clone)]
pub struct InterruptController<'a> {
    node: NodeBase<'a>,
}

impl<'a> InterruptController<'a> {
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
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
        let mut st = f.debug_struct("InterruptController");
        st.field("name", &self.name());
        st.finish()
    }
}

impl<'a> Deref for InterruptController<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
