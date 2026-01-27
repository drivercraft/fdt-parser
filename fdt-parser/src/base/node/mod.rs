//! Device tree node types and accessors.
//!
//! This module provides the `Node` enum and related types for accessing
//! device tree nodes. Nodes are automatically classified into specialized
//! types (Chosen, Memory, InterruptController, etc.) based on their properties.

use core::ops::Deref;

use super::Fdt;
use crate::{
    base::NodeIter,
    data::{Buffer, Raw, U32Iter2D},
    property::PropIter,
    FdtError, FdtRangeSilce, FdtReg, Phandle, Property, Status,
};

mod chosen;
mod interrupt_controller;
mod memory;

pub use chosen::*;
pub use interrupt_controller::*;
pub use memory::*;

/// Base node type representing any device tree node.
///
/// `NodeBase` provides common functionality available on all nodes,
/// including property access, child iteration, and parent references.
#[derive(Clone)]
pub struct NodeBase<'a> {
    name: &'a str,
    pub(crate) fdt: Fdt<'a>,
    /// The depth/level of this node in the device tree (0 for root)
    pub level: usize,
    pub(crate) raw: Raw<'a>,
    pub(crate) parent: Option<ParentInfo<'a>>,
    interrupt_parent: Option<Phandle>,
}

/// Information about a node's parent, used for address translation.
#[derive(Clone)]
pub(crate) struct ParentInfo<'a> {
    pub name: &'a str,
    pub level: usize,
    pub raw: Raw<'a>,
    /// Parent's #address-cells and #size-cells (for parsing reg)
    pub address_cells: Option<u8>,
    pub size_cells: Option<u8>,
    /// Parent's ranges for address translation
    pub ranges: Option<FdtRangeSilce<'a>>,
}

/// Builder for creating NodeBase with parent information.
///
/// This struct reduces the number of parameters needed for `NodeBase::new_with_parent_info`
/// by grouping related parameters together.
pub(crate) struct ParentInfoBuilder<'a> {
    pub parent_address_cells: Option<u8>,
    pub parent_size_cells: Option<u8>,
    pub parent_ranges: Option<FdtRangeSilce<'a>>,
    pub interrupt_parent: Option<Phandle>,
}

impl<'a> NodeBase<'a> {
    /// Create a new NodeBase with pre-calculated parent information from the stack.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_parent_info(
        name: &'a str,
        fdt: Fdt<'a>,
        raw: Raw<'a>,
        level: usize,
        parent: Option<&NodeBase<'a>>,
        parent_info: ParentInfoBuilder<'a>,
    ) -> Self {
        let name = if name.is_empty() { "/" } else { name };
        NodeBase {
            name,
            fdt,
            level,
            parent: parent.map(|p| ParentInfo {
                name: p.name(),
                level: p.level(),
                raw: p.raw(),
                address_cells: parent_info.parent_address_cells,
                size_cells: parent_info.parent_size_cells,
                ranges: parent_info.parent_ranges,
            }),
            interrupt_parent: parent_info.interrupt_parent,
            raw,
        }
    }

    /// Returns the name of this node's parent.
    pub fn parent_name(&self) -> Option<&'a str> {
        self.parent_fast().map(|p| p.name())
    }

    /// Returns the parent node as a `Node`.
    pub fn parent(&self) -> Option<Node<'a>> {
        let parent_info = self.parent.as_ref()?;
        self.fdt
            .all_nodes()
            .flatten()
            .find(|node| node.name() == parent_info.name && node.level() == parent_info.level)
    }

    pub(crate) fn parent_fast(&self) -> Option<NodeBase<'a>> {
        self.parent.as_ref().map(|p| NodeBase {
            name: p.name,
            fdt: self.fdt.clone(),
            level: p.level,
            raw: p.raw,
            parent: None,
            interrupt_parent: None,
        })
    }

    /// Returns the raw data for this node.
    pub fn raw(&self) -> Raw<'a> {
        self.raw
    }

    /// Get the name of this node.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Get the level/depth of this node in the device tree.
    pub fn level(&self) -> usize {
        self.level
    }

    /// Get compatible strings for this node (placeholder implementation).
    pub fn compatibles(&self) -> Result<impl Iterator<Item = &'a str> + 'a, FdtError> {
        let prop = self.find_property("compatible")?;
        Ok(prop.str_list())
    }

    /// Returns a flattened iterator over compatible strings.
    ///
    /// This is an alias for [`compatibles`](Self::compatibles) that
    /// returns the same iterator for chaining with other iterator operations.
    pub fn compatibles_flatten(&self) -> Result<impl Iterator<Item = &'a str> + 'a, FdtError> {
        self.compatibles()
    }

    /// Returns an iterator over this node's register entries.
    ///
    /// The addresses are automatically translated from child bus addresses
    /// to parent bus addresses using the parent's ranges property.
    pub fn reg(&self) -> Result<RegIter<'a>, FdtError> {
        let prop = self.find_property("reg")?;

        // Get parent info from ParentInfo structure
        let parent_info = self
            .parent
            .as_ref()
            .ok_or(FdtError::NodeNotFound("parent"))?;

        // reg parsing uses the immediate parent's cells
        let address_cell = parent_info.address_cells.unwrap_or(2);
        let size_cell = parent_info.size_cells.unwrap_or(1);

        // Use parent's pre-calculated ranges for address translation
        let ranges = parent_info.ranges.clone();

        Ok(RegIter {
            size_cell,
            address_cell,
            buff: prop.data.buffer(),
            ranges,
        })
    }

    fn is_interrupt_controller(&self) -> bool {
        self.find_property("#interrupt-controller").is_ok()
    }

    /// Check if this node is the root node.
    pub fn is_root(&self) -> bool {
        self.level == 0
    }

    /// Get debug information about the node (for debugging purposes only).
    pub fn debug_info(&self) -> NodeDebugInfo<'a> {
        NodeDebugInfo {
            name: self.name(),
            level: self.level,
            pos: self.raw.pos(),
        }
    }

    /// Returns an iterator over this node's properties.
    pub fn properties(&self) -> impl Iterator<Item = Result<Property<'a>, FdtError>> + '_ {
        let reader = self.raw.buffer();
        PropIter::new(self.fdt.clone(), reader)
    }

    /// Find a property by name.
    pub fn find_property(&self, name: &str) -> Result<Property<'a>, FdtError> {
        for prop in self.properties() {
            let prop = prop?;
            if prop.name.eq(name) {
                return Ok(prop);
            }
        }
        Err(FdtError::NotFound)
    }

    /// Get this node's phandle.
    pub fn phandle(&self) -> Result<Phandle, FdtError> {
        let prop = self.find_property("phandle")?;
        Ok(prop.u32()?.into())
    }

    /// Find [InterruptController] from current node or its parent.
    pub fn interrupt_parent(&self) -> Result<InterruptController<'a>, FdtError> {
        // First try to get the interrupt parent phandle from the node itself
        let phandle = self.interrupt_parent.ok_or(FdtError::NotFound)?;

        // Find the node with this phandle
        let node = self.fdt.get_node_by_phandle(phandle)?;
        match node {
            Node::InterruptController(ic) => Ok(ic),
            _ => Err(FdtError::NodeNotFound("interrupt-parent")),
        }
    }

    /// Get the interrupt parent phandle for this node.
    pub fn get_interrupt_parent_phandle(&self) -> Option<Phandle> {
        self.interrupt_parent
    }

    /// Returns an iterator over this node's interrupts.
    ///
    /// Each interrupt is represented as an iterator of u32 cells.
    pub fn interrupts(
        &self,
    ) -> Result<impl Iterator<Item = impl Iterator<Item = u32> + 'a> + 'a, FdtError> {
        let prop = self.find_property("interrupts")?;
        let irq_parent = self.interrupt_parent()?;
        let cell_size = irq_parent.interrupt_cells()?;
        let iter = U32Iter2D::new(&prop.data, cell_size);

        Ok(iter)
    }

    /// Get the clock-frequency property value.
    pub fn clock_frequency(&self) -> Result<u32, FdtError> {
        let prop = self.find_property("clock-frequency")?;
        prop.u32()
    }

    /// Returns an iterator over this node's children.
    pub fn children(&self) -> NodeChildIter<'a> {
        NodeChildIter {
            fdt: self.fdt.clone(),
            parent: self.clone(),
            all_nodes: None,
            target_level: 0,
            found_parent: false,
        }
    }

    /// Get the status property value.
    pub fn status(&self) -> Result<Status, FdtError> {
        let prop = self.find_property("status")?;
        let s = prop.str()?;

        if s.contains("disabled") {
            return Ok(Status::Disabled);
        }

        if s.contains("okay") {
            return Ok(Status::Okay);
        }

        Err(FdtError::NotFound)
    }
}

/// Node debug information.
#[derive(Debug)]
pub struct NodeDebugInfo<'a> {
    /// The name of the node
    pub name: &'a str,
    /// The depth/level of the node in the device tree
    pub level: usize,
    /// The position of the node in the raw data
    pub pos: usize,
}

impl core::fmt::Debug for NodeBase<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Node").field("name", &self.name()).finish()
    }
}

/// Iterator over register entries.
pub struct RegIter<'a> {
    pub(crate) size_cell: u8,
    pub(crate) address_cell: u8,
    pub(crate) buff: Buffer<'a>,
    pub(crate) ranges: Option<FdtRangeSilce<'a>>,
}
impl Iterator for RegIter<'_> {
    type Item = FdtReg;

    fn next(&mut self) -> Option<Self::Item> {
        let child_bus_address = self.buff.take_by_cell_size(self.address_cell)?;

        let mut address = child_bus_address;

        if let Some(ranges) = &self.ranges {
            for one in ranges.iter() {
                let range_child_bus_address = one.child_bus_address().as_u64();
                let range_parent_bus_address = one.parent_bus_address().as_u64();

                if child_bus_address >= range_child_bus_address
                    && child_bus_address < range_child_bus_address + one.size
                {
                    address =
                        child_bus_address - range_child_bus_address + range_parent_bus_address;
                    break;
                }
            }
        }

        let size = if self.size_cell > 0 {
            Some(self.buff.take_by_cell_size(self.size_cell)? as usize)
        } else {
            None
        };
        Some(FdtReg {
            address,
            child_bus_address,
            size,
        })
    }
}

/// Typed node enum for specialized node access.
///
/// Nodes are automatically classified based on their name and properties.
/// Use pattern matching to access node-specific functionality.
#[derive(Debug, Clone)]
pub enum Node<'a> {
    /// A general-purpose node without special handling
    General(NodeBase<'a>),
    /// The /chosen node containing boot parameters
    Chosen(Chosen<'a>),
    /// A memory node (e.g., /memory@0)
    Memory(Memory<'a>),
    /// An interrupt controller node
    InterruptController(InterruptController<'a>),
}

impl<'a> Node<'a> {
    /// Returns a reference to the underlying `NodeBase`.
    pub fn node(&self) -> &NodeBase<'a> {
        self.deref()
    }
}

impl<'a> From<NodeBase<'a>> for Node<'a> {
    fn from(node: NodeBase<'a>) -> Self {
        if node.name() == "chosen" {
            Node::Chosen(Chosen::new(node))
        } else if node.name().starts_with("memory@") {
            Node::Memory(Memory::new(node))
        } else if node.is_interrupt_controller() {
            Node::InterruptController(InterruptController::new(node))
        } else {
            Node::General(node)
        }
    }
}

impl<'a> Deref for Node<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            Node::General(n) => n,
            Node::Chosen(n) => n,
            Node::Memory(n) => n,
            Node::InterruptController(n) => n,
        }
    }
}

/// Iterator over a node's children.
pub struct NodeChildIter<'a> {
    fdt: Fdt<'a>,
    parent: NodeBase<'a>,
    all_nodes: Option<NodeIter<'a, 16>>,
    target_level: usize,
    found_parent: bool,
}

impl<'a> Iterator for NodeChildIter<'a> {
    type Item = Result<Node<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Lazily initialize the node iterator
        if self.all_nodes.is_none() {
            self.all_nodes = Some(self.fdt.all_nodes());
        }

        let all_nodes = self.all_nodes.as_mut()?;

        // Search for child nodes
        loop {
            let node = match all_nodes.next()? {
                Ok(node) => node,
                Err(e) => return Some(Err(e)),
            };

            // First, find the parent node
            if !self.found_parent {
                if node.name() == self.parent.name() && node.level() == self.parent.level() {
                    self.found_parent = true;
                    self.target_level = node.level() + 1;
                }
                continue;
            }

            // Parent node found, now look for child nodes
            let current_level = node.level();

            // If current node's level equals target level and follows parent in tree structure,
            // then it's a direct child of the parent node
            if current_level == self.target_level {
                return Some(Ok(node));
            }

            // If current node's level is less than or equal to parent's level,
            // we've left the parent's subtree
            if current_level <= self.parent.level() {
                break;
            }
        }

        None
    }
}

impl<'a> NodeChildIter<'a> {
    /// Create a new child node iterator.
    pub fn new(fdt: Fdt<'a>, parent: NodeBase<'a>) -> Self {
        NodeChildIter {
            fdt,
            parent,
            all_nodes: None,
            target_level: 0,
            found_parent: false,
        }
    }

    /// Get a reference to the parent node.
    pub fn parent(&self) -> &NodeBase<'a> {
        &self.parent
    }

    /// Collect all child nodes into a Vec.
    pub fn collect_children(self) -> Result<alloc::vec::Vec<Node<'a>>, FdtError> {
        self.collect()
    }

    /// Find a child node by name.
    pub fn find_child_by_name(self, name: &str) -> Result<Node<'a>, FdtError> {
        for child_result in self {
            let child = child_result?;
            if child.name() == name {
                return Ok(child);
            }
        }
        Err(FdtError::NotFound)
    }

    /// Find a child node by compatible string.
    pub fn find_child_by_compatible(self, compatible: &str) -> Result<Node<'a>, FdtError> {
        for child_result in self {
            let child = child_result?;
            match child.compatibles() {
                Ok(mut compatibles) => {
                    if compatibles.any(|comp| comp == compatible) {
                        return Ok(child);
                    }
                }
                Err(FdtError::NotFound) => {}
                Err(e) => return Err(e),
            }
        }
        Err(FdtError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::{Fdt, FdtError};

    #[test]
    fn test_node_child_iter_basic() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // Find the root node
        let root_node = fdt.find_nodes("/").next().unwrap().unwrap();

        // Test child node iterator
        let children: Result<alloc::vec::Vec<_>, _> = root_node.children().collect();
        let children = children.unwrap();

        // Root node should have children
        assert!(!children.is_empty(), "Root node should have children");

        // All children should be at level 1
        for child in &children {
            assert_eq!(
                child.level(),
                1,
                "Root node's direct children should be at level 1"
            );
        }

        // Check that expected children are present
        let child_names: alloc::vec::Vec<_> = children.iter().map(|c| c.name()).collect();
        assert!(
            child_names.contains(&"chosen"),
            "Should contain chosen node"
        );
        assert!(
            child_names.contains(&"memory@0"),
            "Should contain memory@0 node"
        );
    }

    #[test]
    fn test_find_child_by_name() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // Find the root node
        let root_node = fdt.find_nodes("/").next().unwrap().unwrap();

        // Test finding child by name
        let memory_node = root_node.children().find_child_by_name("memory@0").unwrap();

        assert_eq!(memory_node.name(), "memory@0");

        // Test finding non-existent node
        let nonexistent_err = root_node
            .children()
            .find_child_by_name("nonexistent")
            .unwrap_err();
        assert!(matches!(nonexistent_err, FdtError::NotFound));
    }

    #[test]
    fn test_child_iter_empty() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // Find a leaf node (a node with no children)
        let leaf_node = fdt.find_nodes("/chosen").next().unwrap().unwrap();

        // Test leaf node's child iterator
        let children: Result<alloc::vec::Vec<_>, _> = leaf_node.children().collect();
        let children = children.unwrap();

        assert!(children.is_empty(), "Leaf node should not have children");
    }

    #[test]
    fn test_child_iter_multiple_levels() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/bcm2711-rpi-4-b.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // Find reserved-memory node, which should have children
        let reserved_memory = fdt
            .all_nodes()
            .find(|node| node.as_ref().is_ok_and(|n| n.name() == "reserved-memory"))
            .unwrap()
            .unwrap();

        // Test child node iterator
        let children: Result<alloc::vec::Vec<_>, _> = reserved_memory.children().collect();
        let children = children.unwrap();

        // Ensure children's level is correct
        for child in &children {
            assert_eq!(
                child.level(),
                reserved_memory.level() + 1,
                "Child's level should be 1 higher than parent's level"
            );
        }
    }
}
