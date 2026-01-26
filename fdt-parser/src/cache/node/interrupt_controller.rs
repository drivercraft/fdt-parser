use core::{fmt::Debug, ops::Deref};

use crate::{cache::node::NodeBase, FdtError};

/// An interrupt controller node (cached version).
#[derive(Clone)]
pub struct InterruptController {
    node: NodeBase,
}

impl InterruptController {
    pub(crate) fn new(node: NodeBase) -> Self {
        InterruptController { node }
    }

    /// Get the number of interrupt cells this controller uses
    pub fn interrupt_cells(&self) -> Result<u32, FdtError> {
        match self.node.find_property("#interrupt-cells") {
            Some(prop) => prop.u32(),
            None => Err(FdtError::PropertyNotFound("#interrupt-cells")),
        }
    }
}

impl Debug for InterruptController {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InterruptController")
            .field("name", &self.node.name())
            .field("interrupt_cells", &self.interrupt_cells())
            .finish()
    }
}

impl Deref for InterruptController {
    type Target = NodeBase;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
