use core::ops::Deref;

use alloc::vec::Vec;

use crate::node::gerneric::NodeRefGen;

/// Interrupt controller node reference.
///
/// Provides specialized access to interrupt controller nodes and their properties.
#[derive(Clone)]
pub struct NodeRefInterruptController<'a> {
    /// The underlying generic node reference
    pub node: NodeRefGen<'a>,
}

impl<'a> NodeRefInterruptController<'a> {
    /// Attempts to create an interrupt controller reference from a generic node.
    ///
    /// Returns `Err` with the original node if it's not an interrupt controller.
    pub fn try_from(node: NodeRefGen<'a>) -> Result<Self, NodeRefGen<'a>> {
        if !is_interrupt_controller_node(&node) {
            return Err(node);
        }
        Ok(Self { node })
    }

    /// Get #interrupt-cells value
    ///
    /// This determines how many cells are needed to describe interrupts
    /// referencing this controller
    pub fn interrupt_cells(&self) -> Option<u32> {
        self.find_property("#interrupt-cells")
            .and_then(|prop| prop.get_u32())
    }

    /// Get #address-cells value (used for interrupt-map)
    pub fn interrupt_address_cells(&self) -> Option<u32> {
        self.find_property("#address-cells")
            .and_then(|prop| prop.get_u32())
    }

    /// Check if this is an interrupt controller
    pub fn is_interrupt_controller(&self) -> bool {
        // Check for interrupt-controller property (empty property marker)
        self.find_property("interrupt-controller").is_some()
    }

    /// Get compatible list
    pub fn compatibles(&self) -> Vec<&str> {
        self.node.compatibles().collect()
    }
}

impl<'a> Deref for NodeRefInterruptController<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

/// Check if node is an interrupt controller
fn is_interrupt_controller_node(node: &NodeRefGen) -> bool {
    // Name starts with interrupt-controller
    if node.name().starts_with("interrupt-controller") {
        return true;
    }

    // Or has interrupt-controller property
    node.find_property("interrupt-controller").is_some()
}
