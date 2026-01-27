//! Context for FDT traversal and node lookup.
//!
//! This module provides the `Context` type which maintains state during
//! FDT parsing and traversal, including parent references, phandle mappings,
//! and inherited properties like address-cells and size-cells.

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use fdt_raw::{Phandle, Status};

use crate::{Node, RangesEntry};

// ============================================================================
// FDT Context
// ============================================================================

/// Traversal context storing parent node reference stack.
///
/// The context maintains state during FDT parsing and tree traversal,
/// including the stack of parent nodes from root to the current position
/// and mappings for efficient node lookups by phandle.
#[derive(Clone, Default)]
pub struct Context<'a> {
    /// Parent node reference stack (from root to current node's parent)
    /// The stack bottom is the root node, the stack top is the direct parent
    pub parents: Vec<&'a Node>,

    /// Phandle to node reference mapping
    /// Used for fast node lookup by phandle (e.g., interrupt parent)
    pub phandle_map: BTreeMap<Phandle, &'a Node>,
}

impl<'a> Context<'a> {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current path as a string.
    pub fn current_path(&self) -> String {
        self.parents
            .iter()
            .map(|n| n.name())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Creates a context for the root node.
    pub fn for_root() -> Self {
        Self::default()
    }

    /// Returns the current depth (parent count + 1).
    pub fn depth(&self) -> usize {
        self.parents.len() + 1
    }

    /// Returns the direct parent node.
    pub fn parent(&self) -> Option<&'a Node> {
        self.parents.last().copied()
    }

    /// Returns the parent's #address-cells value.
    ///
    /// Gets the value from the direct parent node, or returns 2 as default.
    pub fn parent_address_cells(&self) -> u32 {
        self.parent().and_then(|p| p.address_cells()).unwrap_or(2)
    }

    /// Returns the parent's #size-cells value.
    ///
    /// Gets the value from the direct parent node, or returns 1 as default.
    pub fn parent_size_cells(&self) -> u32 {
        self.parent().and_then(|p| p.size_cells()).unwrap_or(1)
    }

    /// Finds the interrupt parent phandle.
    ///
    /// Searches upward through the parent stack to find the nearest
    /// interrupt-parent property.
    pub fn interrupt_parent(&self) -> Option<Phandle> {
        for parent in self.parents.iter().rev() {
            if let Some(phandle) = parent.interrupt_parent() {
                return Some(phandle);
            }
        }
        None
    }

    /// Checks if the node is disabled.
    ///
    /// Returns true if any parent in the stack has status = "disabled".
    pub fn is_disabled(&self) -> bool {
        for parent in &self.parents {
            if matches!(parent.status(), Some(Status::Disabled)) {
                return true;
            }
        }
        false
    }

    /// Collects ranges from all parent nodes for address translation.
    ///
    /// Returns a stack of ranges from root to parent, used for translating
    /// device addresses to CPU physical addresses.
    pub fn collect_ranges(&self) -> Vec<Vec<RangesEntry>> {
        let mut ranges_stack = Vec::new();
        let mut prev_address_cells = 2; // Root node default

        for parent in &self.parents {
            if let Some(ranges) = parent.ranges(prev_address_cells) {
                ranges_stack.push(ranges);
            }
            // Update address cells to current node's value for next level
            prev_address_cells = parent.address_cells().unwrap_or(2);
        }

        ranges_stack
    }

    /// Returns the most recent ranges layer (for current node's address translation).
    pub fn current_ranges(&self) -> Option<Vec<RangesEntry>> {
        // Need parent node to get ranges
        if self.parents.is_empty() {
            return None;
        }

        let parent = self.parents.last()?;

        // Get parent node's parent's address_cells
        let grandparent_address_cells = if self.parents.len() >= 2 {
            self.parents[self.parents.len() - 2]
                .address_cells()
                .unwrap_or(2)
        } else {
            2 // Root node default
        };
        parent.ranges(grandparent_address_cells)
    }

    /// Pushes a node onto the parent stack.
    pub fn push(&mut self, node: &'a Node) {
        self.parents.push(node);
    }

    /// Finds a node by its phandle value.
    pub fn find_by_phandle(&self, phandle: Phandle) -> Option<&'a Node> {
        self.phandle_map.get(&phandle).copied()
    }

    /// Builds a phandle mapping from a node tree.
    pub fn build_phandle_map_from_node(node: &'a Node, map: &mut BTreeMap<Phandle, &'a Node>) {
        if let Some(phandle) = node.phandle() {
            map.insert(phandle, node);
        }
        for child in node.children() {
            Self::build_phandle_map_from_node(child, map);
        }
    }
}
