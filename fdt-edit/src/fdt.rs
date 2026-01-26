//! Editable Flattened Device Tree (FDT) structure.
//!
//! This module provides the main `Fdt` type for creating, modifying, and
//! encoding device tree blobs. It supports loading from existing DTB files,
//! building new trees programmatically, and applying device tree overlays.

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

pub use fdt_raw::MemoryReservation;
use fdt_raw::{FdtError, Phandle, Status};

use crate::{
    ClockType, Node, NodeIter, NodeIterMut, NodeKind, NodeMut, NodeRef,
    encode::{FdtData, FdtEncoder},
};

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

        // Build phandle cache
        fdt.rebuild_phandle_cache();

        Ok(fdt)
    }

    /// Rebuilds the phandle cache by scanning all nodes.
    pub fn rebuild_phandle_cache(&mut self) {
        self.phandle_cache.clear();
        let root_clone = self.root.clone();
        self.build_phandle_cache_recursive(&root_clone, "/");
    }

    /// Recursively builds the phandle cache starting from a node.
    fn build_phandle_cache_recursive(&mut self, node: &Node, current_path: &str) {
        // Check if node has a phandle property
        if let Some(phandle) = node.phandle() {
            self.phandle_cache.insert(phandle, current_path.to_string());
        }

        // Recursively process child nodes
        for child in node.children() {
            let child_name = child.name();
            let child_path = if current_path == "/" {
                format!("/{}", child_name)
            } else {
                format!("{}/{}", current_path, child_name)
            };
            self.build_phandle_cache_recursive(child, &child_path);
        }
    }

    /// Normalizes a path: resolves aliases or ensures leading '/'.
    fn normalize_path(&self, path: &str) -> Option<String> {
        if path.starts_with('/') {
            Some(path.to_string())
        } else {
            // Try to resolve as an alias
            self.resolve_alias(path).map(|s| s.to_string())
        }
    }

    /// Resolves an alias to its full path.
    ///
    /// Looks up the alias in the /aliases node and returns the
    /// corresponding path string.
    pub fn resolve_alias(&self, alias: &str) -> Option<&str> {
        let aliases_node = self.get_by_path("/aliases")?;
        let prop = aliases_node.find_property(alias)?;
        prop.as_str()
    }

    /// Returns all aliases as (name, path) pairs.
    pub fn aliases(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if let Some(aliases_node) = self.get_by_path("/aliases") {
            for prop in aliases_node.properties() {
                let name = prop.name().to_string();
                let path = prop.as_str().unwrap().to_string();
                result.push((name, path));
            }
        }
        result
    }

    /// Finds a node by its phandle value.
    pub fn find_by_phandle(&self, phandle: Phandle) -> Option<NodeRef<'_>> {
        let path = self.phandle_cache.get(&phandle)?.clone();
        self.get_by_path(&path)
    }

    /// Finds a node by phandle (mutable reference).
    pub fn find_by_phandle_mut(&mut self, phandle: Phandle) -> Option<NodeMut<'_>> {
        let path = self.phandle_cache.get(&phandle)?.clone();
        self.get_by_path_mut(&path)
    }

    /// Returns the root node.
    pub fn root<'a>(&'a self) -> NodeRef<'a> {
        self.get_by_path("/").unwrap()
    }

    /// Returns the root node (mutable reference).
    pub fn root_mut<'a>(&'a mut self) -> NodeMut<'a> {
        self.get_by_path_mut("/").unwrap()
    }

    /// Applies a device tree overlay to this FDT.
    ///
    /// Supports two overlay formats:
    /// 1. Fragment format: contains fragment@N nodes with target/target-path and __overlay__
    /// 2. Simple format: directly contains __overlay__ node
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Fragment format
    /// fragment@0 {
    ///     target-path = "/soc";
    ///     __overlay__ {
    ///         new_node { ... };
    ///     };
    /// };
    /// ```
    pub fn apply_overlay(&mut self, overlay: &Fdt) -> Result<(), FdtError> {
        // Iterate through all children of overlay root node
        for child in overlay.root.children() {
            if child.name().starts_with("fragment@") || child.name() == "fragment" {
                // Fragment format
                self.apply_fragment(child)?;
            } else if child.name() == "__overlay__" {
                // Simple format: apply directly to root
                self.merge_overlay_to_root(child)?;
            } else if child.name() == "__symbols__"
                || child.name() == "__fixups__"
                || child.name() == "__local_fixups__"
            {
                // Skip these special nodes
                continue;
            }
        }

        // Rebuild phandle cache
        self.rebuild_phandle_cache();

        Ok(())
    }

    /// Applies a single fragment from an overlay.
    fn apply_fragment(&mut self, fragment: &Node) -> Result<(), FdtError> {
        // Get target path
        let target_path = self.resolve_fragment_target(fragment)?;

        // Find __overlay__ child node
        let overlay_node = fragment
            .get_child("__overlay__")
            .ok_or(FdtError::NotFound)?;

        // Find target node and apply overlay
        let target_path_owned = target_path.to_string();

        // Apply overlay to target node
        self.apply_overlay_to_target(&target_path_owned, overlay_node)?;

        Ok(())
    }

    /// Resolves the target path of a fragment.
    fn resolve_fragment_target(&self, fragment: &Node) -> Result<String, FdtError> {
        // Prefer target-path (string path)
        if let Some(prop) = fragment.get_property("target-path") {
            return Ok(prop.as_str().ok_or(FdtError::Utf8Parse)?.to_string());
        }

        // Use target (phandle reference)
        if let Some(prop) = fragment.get_property("target") {
            let ph = prop.get_u32().ok_or(FdtError::InvalidInput)?;
            let ph = Phandle::from(ph);

            // Find node by phandle and build path
            if let Some(node) = self.find_by_phandle(ph) {
                return Ok(node.path());
            }
        }

        Err(FdtError::NotFound)
    }

    /// Applies an overlay to a target node.
    fn apply_overlay_to_target(
        &mut self,
        target_path: &str,
        overlay_node: &Node,
    ) -> Result<(), FdtError> {
        // Find target node
        let mut target = self
            .get_by_path_mut(target_path)
            .ok_or(FdtError::NotFound)?;

        // Merge overlay properties and child nodes
        Self::merge_nodes(target.node, overlay_node);

        Ok(())
    }

    /// Merges an overlay node to the root node.
    fn merge_overlay_to_root(&mut self, overlay: &Node) -> Result<(), FdtError> {
        // Merge properties and child nodes to root
        for prop in overlay.properties() {
            self.root.set_property(prop.clone());
        }

        for child in overlay.children() {
            let child_name = child.name();
            if let Some(existing) = self.root.get_child_mut(child_name) {
                // Merge into existing child node
                Self::merge_nodes(existing, child);
            } else {
                // Add new child node
                self.root.add_child(child.clone());
            }
        }

        Ok(())
    }

    /// Recursively merges two nodes.
    fn merge_nodes(target: &mut Node, source: &Node) {
        // Merge properties (source overrides target)
        for prop in source.properties() {
            target.set_property(prop.clone());
        }

        // Merge child nodes
        for source_child in source.children() {
            let child_name = &source_child.name();
            if let Some(target_child) = target.get_child_mut(child_name) {
                // Recursive merge
                Self::merge_nodes(target_child, source_child);
            } else {
                // Add new child node
                target.add_child(source_child.clone());
            }
        }
    }

    /// Applies an overlay with optional deletion of disabled nodes.
    ///
    /// If a node in the overlay has status = "disabled", the corresponding
    /// target node will be disabled or deleted.
    pub fn apply_overlay_with_delete(
        &mut self,
        overlay: &Fdt,
        delete_disabled: bool,
    ) -> Result<(), FdtError> {
        self.apply_overlay(overlay)?;

        if delete_disabled {
            // Remove all nodes with status = "disabled"
            Self::remove_disabled_nodes(&mut self.root);
            self.rebuild_phandle_cache();
        }

        Ok(())
    }

    /// Recursively removes disabled nodes.
    fn remove_disabled_nodes(node: &mut Node) {
        // Remove disabled child nodes
        let mut to_remove = Vec::new();
        for child in node.children() {
            if matches!(child.status(), Some(Status::Disabled)) {
                to_remove.push(child.name().to_string());
            }
        }

        for child_name in to_remove {
            node.remove_child(&child_name);
        }

        // Recursively process remaining child nodes
        for child in node.children_mut() {
            Self::remove_disabled_nodes(child);
        }
    }

    /// Removes a node by exact path.
    ///
    /// Supports exact path matching only. Aliases are automatically resolved.
    ///
    /// # Arguments
    ///
    /// * `path` - Node path (e.g., "soc/gpio@1000", "/soc/gpio@1000", or an alias)
    ///
    /// # Returns
    ///
    /// * `Ok(Some(node))` - The removed node
    /// * `Ok(None)` - Path not found
    /// * `Err(FdtError)` - Invalid path format
    ///
    /// # Example
    ///
    /// ```rust
    /// # use fdt_edit::{Fdt, Node};
    /// let mut fdt = Fdt::new();
    ///
    /// // Add node then remove it
    /// let mut soc = Node::new("soc");
    /// soc.add_child(Node::new("gpio@1000"));
    /// fdt.root.add_child(soc);
    ///
    /// // Remove node with exact path
    /// let removed = fdt.remove_node("/soc/gpio@1000")?;
    /// assert!(removed.is_some());
    /// # Ok::<(), fdt_raw::FdtError>(())
    /// ```
    pub fn remove_node(&mut self, path: &str) -> Result<Option<Node>, FdtError> {
        let normalized_path = self.normalize_path(path).ok_or(FdtError::InvalidInput)?;

        // Use exact path for removal
        let result = self.root.remove_by_path(&normalized_path)?;

        // If removal succeeded but result is None, path doesn't exist
        if result.is_none() {
            return Err(FdtError::NotFound);
        }

        Ok(result)
    }

    /// Returns a depth-first iterator over all nodes.
    pub fn all_nodes(&self) -> impl Iterator<Item = NodeRef<'_>> + '_ {
        NodeIter::new(&self.root)
    }

    /// Returns a mutable depth-first iterator over all nodes.
    pub fn all_nodes_mut(&mut self) -> impl Iterator<Item = NodeMut<'_>> + '_ {
        NodeIterMut::new(&mut self.root)
    }

    /// Finds nodes by path (supports fuzzy matching).
    pub fn find_by_path<'a>(&'a self, path: &str) -> impl Iterator<Item = NodeRef<'a>> {
        let path = self
            .normalize_path(path)
            .unwrap_or_else(|| path.to_string());

        NodeIter::new(&self.root).filter_map(move |node_ref| {
            if node_ref.path_eq_fuzzy(&path) {
                Some(node_ref)
            } else {
                None
            }
        })
    }

    /// Gets a node by exact path.
    pub fn get_by_path<'a>(&'a self, path: &str) -> Option<NodeRef<'a>> {
        let path = self.normalize_path(path)?;
        NodeIter::new(&self.root).find_map(move |node_ref| {
            if node_ref.path_eq(&path) {
                Some(node_ref)
            } else {
                None
            }
        })
    }

    /// Gets a node by exact path (mutable reference).
    pub fn get_by_path_mut<'a>(&'a mut self, path: &str) -> Option<NodeMut<'a>> {
        let path = self.normalize_path(path)?;
        NodeIterMut::new(&mut self.root).find_map(move |node_mut| {
            if node_mut.path_eq(&path) {
                Some(node_mut)
            } else {
                None
            }
        })
    }

    /// Finds nodes with matching compatible strings.
    pub fn find_compatible(&self, compatible: &[&str]) -> Vec<NodeRef<'_>> {
        let mut results = Vec::new();
        for node_ref in self.all_nodes() {
            let Some(ls) = node_ref.compatible() else {
                continue;
            };

            for comp in ls {
                if compatible.contains(&comp) {
                    results.push(node_ref);
                    break;
                }
            }
        }
        results
    }

    /// Serializes the FDT to binary DTB format.
    pub fn encode(&self) -> FdtData {
        FdtEncoder::new(self).encode()
    }
}

impl core::fmt::Display for Fdt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Output DTS header
        writeln!(f, "/dts-v1/;")?;

        // Output memory reservation block
        for reservation in &self.memory_reservations {
            writeln!(
                f,
                "/memreserve/ 0x{:x} 0x{:x};",
                reservation.address, reservation.size
            )?;
        }

        // Output root node
        writeln!(f, "{}", self.root)
    }
}

impl core::fmt::Debug for Fdt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            // Deep debug format with node traversal
            self.fmt_debug_deep(f)
        } else {
            // Simple debug format
            f.debug_struct("Fdt")
                .field("boot_cpuid_phys", &self.boot_cpuid_phys)
                .field("memory_reservations_count", &self.memory_reservations.len())
                .field("root_node_name", &self.root.name)
                .field("total_nodes", &self.root.children().len())
                .field("phandle_cache_size", &self.phandle_cache.len())
                .finish()
        }
    }
}

impl Fdt {
    /// Formats the FDT with detailed debug information.
    fn fmt_debug_deep(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Fdt {{")?;
        writeln!(f, "    boot_cpuid_phys: 0x{:x},", self.boot_cpuid_phys)?;
        writeln!(
            f,
            "    memory_reservations_count: {},",
            self.memory_reservations.len()
        )?;
        writeln!(f, "    phandle_cache_size: {},", self.phandle_cache.len())?;
        writeln!(f, "    nodes:")?;

        // Iterate through all nodes and print debug info with indentation
        for (i, node) in self.all_nodes().enumerate() {
            self.fmt_node_debug(f, &node, 2, i)?;
        }

        writeln!(f, "}}")
    }

    /// Formats a single node for debug output.
    fn fmt_node_debug(
        &self,
        f: &mut core::fmt::Formatter<'_>,
        node: &NodeRef,
        indent: usize,
        index: usize,
    ) -> core::fmt::Result {
        // Print indentation
        for _ in 0..indent {
            write!(f, "    ")?;
        }

        // Print node index and basic info
        write!(f, "[{:03}] {}: ", index, node.name())?;

        // Print type-specific information
        match node.as_ref() {
            NodeKind::Clock(clock) => {
                write!(f, "Clock")?;
                if let ClockType::Fixed(fixed) = &clock.kind {
                    write!(f, " (Fixed, {}Hz)", fixed.frequency)?;
                } else {
                    write!(f, " (Provider)")?;
                }
                if !clock.clock_output_names.is_empty() {
                    write!(f, ", outputs: {:?}", clock.clock_output_names)?;
                }
                write!(f, ", cells={}", clock.clock_cells)?;
            }
            NodeKind::Pci(pci) => {
                write!(f, "PCI")?;
                if let Some(bus_range) = pci.bus_range() {
                    write!(f, " (bus: {:?})", bus_range)?;
                }
                write!(f, ", interrupt-cells={}", pci.interrupt_cells())?;
            }
            NodeKind::InterruptController(ic) => {
                write!(f, "InterruptController")?;
                if let Some(cells) = ic.interrupt_cells() {
                    write!(f, " (cells={})", cells)?;
                }
                let compatibles = ic.compatibles();
                if !compatibles.is_empty() {
                    write!(f, ", compatible: {:?}", compatibles)?;
                }
            }
            NodeKind::Memory(mem) => {
                write!(f, "Memory")?;
                let regions = mem.regions();
                if !regions.is_empty() {
                    write!(f, " ({} regions", regions.len())?;
                    for (i, region) in regions.iter().take(2).enumerate() {
                        write!(f, ", [{}]: 0x{:x}+0x{:x}", i, region.address, region.size)?;
                    }
                    if regions.len() > 2 {
                        write!(f, ", ...")?;
                    }
                    write!(f, ")")?;
                }
            }
            NodeKind::Generic(_) => {
                write!(f, "Generic")?;
            }
        }

        // Print phandle information
        if let Some(phandle) = node.phandle() {
            write!(f, ", phandle={}", phandle)?;
        }

        // Print address and size cells information
        if let Some(address_cells) = node.address_cells() {
            write!(f, ", #address-cells={}", address_cells)?;
        }
        if let Some(size_cells) = node.size_cells() {
            write!(f, ", #size-cells={}", size_cells)?;
        }

        writeln!(f)?;

        Ok(())
    }
}
