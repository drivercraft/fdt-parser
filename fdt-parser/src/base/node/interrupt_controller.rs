//! Interrupt controller node type.
//!
//! This module provides the `InterruptController` type for nodes that
//! manage interrupt routing and handling in the system.

use core::ops::Deref;

use super::NodeBase;
use crate::FdtError;

/// An interrupt controller device node.
///
/// Interrupt controllers manage interrupt routing and handling. This type
/// provides access to interrupt controller specific properties like the
/// `#interrupt-cells` property.
#[derive(Clone)]
pub struct InterruptController<'a> {
    node: NodeBase<'a>,
}

impl<'a> InterruptController<'a> {
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        InterruptController { node }
    }

    /// Get the name of this interrupt controller.
    pub fn name(&self) -> &'a str {
        self.node.name()
    }

    /// Get the value of the `#interrupt-cells` property.
    ///
    /// This property specifies the number of cells used to encode an
    /// interrupt specifier for this interrupt controller.
    pub fn interrupt_cells(&self) -> Result<u8, FdtError> {
        let prop = self.node.find_property("#interrupt-cells")?;
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
