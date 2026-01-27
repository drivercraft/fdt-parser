//! Cached node types with specialized accessors.
//!
//! This module provides the `Node` enum and related types for the cached parser.
//! Nodes are automatically classified into specialized types based on their properties.

use core::{fmt::Debug, ops::Deref};

use super::Fdt;
use crate::{
    base::{self, RegIter},
    data::{Raw, U32Iter2D},
    property::PropIter,
    FdtError, FdtRangeSilce, FdtReg, Phandle, Property, Status,
};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

mod chosen;
mod clock;
mod interrupt_controller;
mod memory;
mod pci;

pub use chosen::*;
pub use clock::*;
pub use interrupt_controller::*;
pub use memory::*;
pub use pci::*;

/// Typed node enum for specialized node access.
///
/// Nodes are automatically classified based on their name and properties.
/// Use pattern matching to access node-specific functionality.
#[derive(Debug, Clone)]
pub enum Node {
    /// A general-purpose node without special handling
    General(NodeBase),
    /// The /chosen node containing boot parameters
    Chosen(Chosen),
    /// A memory node (e.g., /memory@0)
    Memory(Memory),
    /// An interrupt controller node
    InterruptController(InterruptController),
    /// A PCI host bridge node
    Pci(Pci),
}

impl Node {
    pub(super) fn new(fdt: &Fdt, meta: &NodeMeta) -> Self {
        let base = NodeBase {
            fdt: fdt.clone(),
            meta: meta.clone(),
        };

        // Create specific type based on node type
        match meta.name.as_str() {
            "chosen" => Self::Chosen(Chosen::new(base)),
            name if name.starts_with("memory@") => Self::Memory(Memory::new(base)),
            _ => {
                // Check if this is a PCI node
                let pci = Pci::new(base.clone());
                if pci.is_pci_host_bridge() {
                    Self::Pci(pci)
                } else if base.is_interrupt_controller() {
                    Self::InterruptController(InterruptController::new(base))
                } else {
                    Self::General(base)
                }
            }
        }
    }
}

impl Deref for Node {
    type Target = NodeBase;

    fn deref(&self) -> &Self::Target {
        match self {
            Node::General(n) => n,
            Node::Chosen(n) => n,
            Node::Memory(n) => n,
            Node::InterruptController(n) => n,
            Node::Pci(n) => n,
        }
    }
}

/// Base node type for cached parser nodes.
///
/// `NodeBase` provides common functionality available on all nodes,
/// with fast lookups using the cached indices.
#[derive(Clone)]
pub struct NodeBase {
    fdt: Fdt,
    meta: NodeMeta,
}

impl NodeBase {
    fn raw<'a>(&'a self) -> Raw<'a> {
        self.fdt.raw().begin_at(self.meta.pos)
    }

    /// Get the level/depth of this node in the device tree.
    pub fn level(&self) -> usize {
        self.meta.level
    }

    /// Get the name of this node.
    pub fn name(&self) -> &str {
        &self.meta.name
    }

    /// Get the full path of this node.
    pub fn full_path(&self) -> &str {
        &self.meta.full_path
    }

    /// Get the parent node.
    pub fn parent(&self) -> Option<Node> {
        let parent_path = self.meta.parent.as_ref()?.path.as_str();
        let parent_meta = self.fdt.inner.get_node_by_path(parent_path)?;
        Some(Node::new(&self.fdt, &parent_meta))
    }

    /// Get all properties of this node.
    pub fn properties<'a>(&'a self) -> Vec<Property<'a>> {
        let reader = self.raw().buffer();
        PropIter::new(self.fdt.fdt_base(), reader)
            .flatten()
            .collect()
    }

    /// Find a property by name.
    pub fn find_property<'a>(&'a self, name: impl AsRef<str>) -> Option<Property<'a>> {
        self.properties()
            .into_iter()
            .find(|prop| prop.name == name.as_ref())
    }

    /// Get compatible strings for this node (placeholder implementation).
    pub fn compatibles(&self) -> Vec<String> {
        self.find_property("compatible")
            .map(|p| {
                p.str_list()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.into())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the status of this node.
    pub fn status(&self) -> Option<Status> {
        self.find_property("status")
            .and_then(|prop| prop.str().ok())
            .and_then(|s| {
                if s.contains("disabled") {
                    Some(Status::Disabled)
                } else if s.contains("okay") {
                    Some(Status::Okay)
                } else {
                    None
                }
            })
    }

    /// Get the #address-cells value for this node.
    pub fn address_cells(&self) -> u8 {
        self.find_property("#address-cells")
            .and_then(|p| p.u32().ok())
            .map(|v| v as u8)
            .or_else(|| {
                self.meta
                    .parent
                    .as_ref()
                    .and_then(|info| info.address_cells)
            })
            .unwrap_or(2)
    }

    fn is_interrupt_controller(&self) -> bool {
        self.name().starts_with("interrupt-controller")
            || self.find_property("interrupt-controller").is_some()
            || self.find_property("#interrupt-controller").is_some()
    }

    /// Get register information for this node.
    ///
    /// Returns a vector of register entries with addresses translated
    /// to the parent bus address space.
    pub fn reg(&self) -> Result<Vec<FdtReg>, FdtError> {
        let prop = self.find_property("reg").ok_or(FdtError::NotFound)?;

        // Get parent info from ParentInfo structure
        let parent_info = self
            .meta
            .parent
            .as_ref()
            .ok_or(FdtError::NodeNotFound("parent"))?;

        // reg parsing uses the immediate parent's cells
        let address_cell = parent_info.address_cells.unwrap_or(2);
        let size_cell = parent_info.size_cells.unwrap_or(1);

        let parent = self.parent().ok_or(FdtError::NodeNotFound("parent"))?;
        let ranges = parent.ranges();
        let iter = RegIter {
            size_cell,
            address_cell,
            buff: prop.data.buffer(),
            ranges,
        };

        Ok(iter.collect())
    }

    /// Get the ranges property for address translation.
    pub fn ranges(&self) -> Option<FdtRangeSilce<'_>> {
        let p = self.find_property("ranges")?;
        let parent_info = self.meta.parent.as_ref();

        let address_cell = self
            .find_property("#address-cells")
            .and_then(|prop| prop.u32().ok())
            .map(|v| v as u8)
            .or_else(|| parent_info.and_then(|info| info.address_cells))
            .unwrap_or(2);

        let size_cell = self
            .find_property("#size-cells")
            .and_then(|prop| prop.u32().ok())
            .map(|v| v as u8)
            .or_else(|| parent_info.and_then(|info| info.size_cells))
            .unwrap_or(1);

        let address_cell_parent = parent_info.and_then(|info| info.address_cells).unwrap_or(2);

        Some(FdtRangeSilce::new(
            address_cell,
            address_cell_parent,
            size_cell,
            &p.data,
        ))
    }

    /// Get the interrupt parent phandle for this node.
    pub fn interrupt_parent_phandle(&self) -> Option<Phandle> {
        self.meta.interrupt_parent
    }

    /// Get the interrupt parent node.
    pub fn interrupt_parent(&self) -> Option<InterruptController> {
        let phandle = self.interrupt_parent_phandle()?;
        let irq = self.fdt.get_node_by_phandle(phandle)?;
        let Node::InterruptController(i) = irq else {
            return None;
        };
        Some(i)
    }

    /// Get the interrupts for this node.
    pub fn interrupts(&self) -> Result<Vec<Vec<u32>>, FdtError> {
        let res = self
            .find_property("interrupts")
            .ok_or(FdtError::PropertyNotFound("interrupts"))?;
        let parent = self
            .interrupt_parent()
            .ok_or(FdtError::PropertyNotFound("interrupt-parent"))?;
        let cells = parent.interrupt_cells()?;
        let iter = U32Iter2D::new(&res.data, cells as _);
        let mut out = Vec::new();
        for entry in iter {
            out.push(entry.collect());
        }
        Ok(out)
    }

    /// Get the clocks used by this node following the Devicetree clock binding.
    pub fn clocks(&self) -> Result<Vec<ClockInfo>, FdtError> {
        let mut clocks = Vec::new();
        let Some(prop) = self.find_property("clocks") else {
            return Ok(clocks);
        };

        let mut data = prop.data.buffer();
        let clock_names: Vec<String> = self
            .find_property("clock-names")
            .map(|p| p.str_list().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let mut index = 0usize;
        while !data.remain().as_ref().is_empty() {
            let phandle_raw = data.take_u32()?;
            let phandle = Phandle::from(phandle_raw);

            let provider = self
                .fdt
                .get_node_by_phandle(phandle)
                .ok_or(FdtError::NodeNotFound("clock"))?;

            let provider_node = provider.deref().clone();
            let clock_cells = provider_node
                .find_property("#clock-cells")
                .and_then(|p| p.u32().ok())
                .unwrap_or(0);
            let select = if clock_cells > 0 {
                data.take_by_cell_size(clock_cells as _)
                    .ok_or(FdtError::BufferTooSmall { pos: data.pos() })?
            } else {
                0
            };

            let provider = ClockType::new(provider_node);
            let provider_output_name = provider.output_name(select);
            let name = clock_names.get(index).cloned();

            clocks.push(ClockInfo {
                name,
                provider_output_name,
                provider,
                phandle,
                select,
            });

            index += 1;
        }

        Ok(clocks)
    }
}

impl Debug for NodeBase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut st = f.debug_struct("NodeBase");
        // st.field("name", &self.name());
        st.finish()
    }
}

/// Metadata for a cached node.
///
/// Contains precomputed information about the node for fast access.
#[derive(Clone)]
pub(super) struct NodeMeta {
    name: String,
    full_path: String,
    pos: usize,
    pub level: usize,
    interrupt_parent: Option<Phandle>,
    parent: Option<ParentInfo>,
}

impl NodeMeta {
    /// Create node metadata from a base parser node.
    pub fn new(node: &base::Node<'_>, full_path: String, parent: Option<&NodeMeta>) -> Self {
        NodeMeta {
            full_path,
            name: node.name().into(),
            pos: node.raw.pos(),
            level: node.level(),
            interrupt_parent: node.get_interrupt_parent_phandle(),
            parent: node.parent.as_ref().map(|p| ParentInfo {
                path: parent.map(|n| n.full_path.clone()).unwrap_or_default(),
                address_cells: p.address_cells,
                size_cells: p.size_cells,
            }),
        }
    }
}

/// Information about a node's parent.
#[derive(Clone)]
struct ParentInfo {
    path: String,
    address_cells: Option<u8>,
    size_cells: Option<u8>,
}
