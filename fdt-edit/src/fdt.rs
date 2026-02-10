//! Editable Flattened Device Tree (FDT) structure.
//!
//! This module provides the main `Fdt` type for creating, modifying, and
//! encoding device tree blobs. It supports loading from existing DTB files,
//! building new trees programmatically, and applying device tree overlays.

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use crate::node_iter::*;
use crate::{FdtError, Phandle};

pub use fdt_raw::MemoryReservation;

use crate::Node;

/// An editable Flattened Device Tree (FDT).
///
/// This structure represents a mutable device tree that can be created from
/// scratch, loaded from an existing DTB file, modified, and encoded back to
/// the binary DTB format. It maintains a phandle cache for efficient node
/// lookups by phandle value.
#[derive(Clone)]
pub struct Fdt {
    /// Boot CPU ID
    pub boot_cpuid_phys: u32,
    /// Memory reservation block entries
    pub memory_reservations: Vec<MemoryReservation>,
    /// Root node of the device tree
    pub root: Node,
    /// Cache mapping phandles to full node paths
    phandle_cache: BTreeMap<Phandle, String>,
}

impl Default for Fdt {
    fn default() -> Self {
        Self::new()
    }
}

impl Fdt {
    /// Creates a new empty FDT.
    pub fn new() -> Self {
        Self {
            boot_cpuid_phys: 0,
            memory_reservations: Vec::new(),
            root: Node::new(""),
            phandle_cache: BTreeMap::new(),
        }
    }

    /// Parses an FDT from raw byte data.
    pub fn from_bytes(data: &[u8]) -> Result<Self, FdtError> {
        let raw_fdt = fdt_raw::Fdt::from_bytes(data)?;
        Self::from_raw(&raw_fdt)
    }

    /// Parses an FDT from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// valid FDT data structure.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Self, FdtError> {
        let raw_fdt = unsafe { fdt_raw::Fdt::from_ptr(ptr)? };
        Self::from_raw(&raw_fdt)
    }

    /// Converts from a raw FDT parser instance.
    fn from_raw(raw_fdt: &fdt_raw::Fdt) -> Result<Self, FdtError> {
        let header = raw_fdt.header();

        let mut fdt = Fdt {
            boot_cpuid_phys: header.boot_cpuid_phys,
            memory_reservations: raw_fdt.memory_reservations().collect(),
            root: Node::new(""),
            phandle_cache: BTreeMap::new(),
        };

        // Build node tree using a stack to track parent nodes
        let mut node_stack: Vec<Node> = Vec::new();

        for raw_node in raw_fdt.all_nodes() {
            let level = raw_node.level();
            let node = Node::from(&raw_node);
            if let Some(phandle) = node.phandle() {
                fdt.phandle_cache
                    .insert(phandle, raw_node.path().to_string());
            }

            // Pop stack until we reach the correct parent level
            while node_stack.len() > level {
                let child = node_stack.pop().unwrap();
                if let Some(parent) = node_stack.last_mut() {
                    parent.add_child(child);
                } else {
                    // This is the root node
                    fdt.root = child;
                }
            }

            node_stack.push(node);
        }

        // Pop all remaining nodes
        while let Some(child) = node_stack.pop() {
            if let Some(parent) = node_stack.last_mut() {
                parent.add_child(child);
            } else {
                // This is the root node
                fdt.root = child;
            }
        }
        Ok(fdt)
    }

    pub fn all_raw_nodes(&self) -> impl Iterator<Item = &Node> {
        self.all_nodes().map(|node_ref| node_ref.node)
    }

    pub fn all_raw_nodes_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.all_nodes_mut().map(|node_ref| node_ref.node)
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = NodeRef<'_>> {
        NodeRefIter::new(&self.root)
    }

    pub fn all_nodes_mut(&mut self) -> impl Iterator<Item = NodeRefMut<'_>> {
        NodeRefIterMut::new(&mut self.root)
    }
}
