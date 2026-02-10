use alloc::{collections::btree_map::BTreeMap, string::String, vec::Vec};
use fdt_raw::{Phandle, Status};

use crate::{Property, RangesEntry};

/// A mutable device tree node.
///
/// Represents a node in the device tree with a name, properties, and child nodes.
/// Provides efficient property and child lookup through cached indices while
/// maintaining insertion order.
#[derive(Clone)]
pub struct Node {
    /// Node name (without path)
    pub name: String,
    /// Property list (maintains original order)
    properties: Vec<Property>,
    /// Property name to index mapping (for fast lookup)
    prop_cache: BTreeMap<String, usize>,
    /// Child nodes
    children: Vec<Node>,
    /// Child name to index mapping (for fast lookup)
    name_cache: BTreeMap<String, usize>,
}

impl Node {
    /// Creates a new node with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            properties: Vec::new(),
            prop_cache: BTreeMap::new(),
            children: Vec::new(),
            name_cache: BTreeMap::new(),
        }
    }

    /// Returns the node's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns an iterator over the node's properties.
    pub fn properties(&self) -> &[Property] {
        &self.properties
    }

    /// Returns a slice of the node's children.
    pub fn children(&self) -> &[Node] {
        &self.children
    }

    /// Returns a mutable iterator over the node's children.
    pub fn children_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.children.iter_mut()
    }

    /// Adds a child node to this node.
    ///
    /// Updates the name cache for fast lookups.
    pub fn add_child(&mut self, child: Node) {
        let index = self.children.len();
        self.name_cache.insert(child.name.clone(), index);
        self.children.push(child);
    }

    /// Adds a property to this node.
    ///
    /// Updates the property cache for fast lookups.
    pub fn add_property(&mut self, prop: Property) {
        let name = prop.name.clone();
        let index = self.properties.len();
        self.prop_cache.insert(name, index);
        self.properties.push(prop);
    }

    /// Gets a child node by name.
    ///
    /// Uses the cache for fast lookup, with a fallback to linear search.
    pub fn get_child(&self, name: &str) -> Option<&Node> {
        if let Some(&index) = self.name_cache.get(name)
            && let Some(child) = self.children.get(index)
        {
            return Some(child);
        }

        // Fallback if the cache is stale
        self.children.iter().find(|c| c.name == name)
    }

    /// Gets a mutable reference to a child node by name.
    ///
    /// Rebuilds the cache on mismatch to keep indices synchronized.
    pub fn get_child_mut(&mut self, name: &str) -> Option<&mut Node> {
        if let Some(&index) = self.name_cache.get(name)
            && index < self.children.len()
            && self.children[index].name == name
        {
            return self.children.get_mut(index);
        }

        // Cache miss or mismatch: search and rebuild cache to keep indices in sync
        let pos = self.children.iter().position(|c| c.name == name)?;
        self.rebuild_name_cache();
        self.children.get_mut(pos)
    }

    /// Removes a child node by name.
    ///
    /// Rebuilds the name cache after removal.
    pub fn remove_child(&mut self, name: &str) -> Option<Node> {
        let index = self
            .name_cache
            .get(name)
            .copied()
            .filter(|&idx| self.children.get(idx).map(|c| c.name.as_str()) == Some(name))
            .or_else(|| self.children.iter().position(|c| c.name == name));

        let idx = index?;

        let removed = self.children.remove(idx);
        self.rebuild_name_cache();
        Some(removed)
    }

    /// Sets a property, adding it if it doesn't exist or updating if it does.
    pub fn set_property(&mut self, prop: Property) {
        let name = prop.name.clone();
        if let Some(&idx) = self.prop_cache.get(&name) {
            // Update existing property
            self.properties[idx] = prop;
        } else {
            // Add new property
            let idx = self.properties.len();
            self.prop_cache.insert(name, idx);
            self.properties.push(prop);
        }
    }

    /// Gets a property by name.
    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.prop_cache.get(name).map(|&idx| &self.properties[idx])
    }

    /// Gets a mutable reference to a property by name.
    pub fn get_property_mut(&mut self, name: &str) -> Option<&mut Property> {
        self.prop_cache
            .get(name)
            .map(|&idx| &mut self.properties[idx])
    }

    fn rebuild_prop_cache(&mut self) {
        self.prop_cache.clear();
        for (idx, prop) in self.properties.iter().enumerate() {
            self.prop_cache.insert(prop.name.clone(), idx);
        }
    }

    /// Removes a property by name.
    ///
    /// Updates indices after removal to keep the cache consistent.
    pub fn remove_property(&mut self, name: &str) -> Option<Property> {
        if let Some(&idx) = self.prop_cache.get(name) {
            // Rebuild indices (need to update subsequent indices after removal)
            let prop = self.properties.remove(idx);
            self.rebuild_prop_cache();
            Some(prop)
        } else {
            None
        }
    }

    /// Returns the `#address-cells` property value.
    pub fn address_cells(&self) -> Option<u32> {
        self.get_property("#address-cells")
            .and_then(|prop| prop.get_u32())
    }

    /// Returns the `#size-cells` property value.
    pub fn size_cells(&self) -> Option<u32> {
        self.get_property("#size-cells")
            .and_then(|prop| prop.get_u32())
    }

    /// Returns the `phandle` property value.
    pub fn phandle(&self) -> Option<Phandle> {
        self.get_property("phandle")
            .and_then(|prop| prop.get_u32())
            .map(Phandle::from)
    }

    /// Returns the `interrupt-parent` property value.
    pub fn interrupt_parent(&self) -> Option<Phandle> {
        self.get_property("interrupt-parent")
            .and_then(|prop| prop.get_u32())
            .map(Phandle::from)
    }

    /// Returns the `status` property value.
    pub fn status(&self) -> Option<Status> {
        let prop = self.get_property("status")?;
        let s = prop.as_str()?;
        match s {
            "okay" => Some(Status::Okay),
            "disabled" => Some(Status::Disabled),
            _ => None,
        }
    }

    /// Parses the `ranges` property for address translation.
    ///
    /// Returns a vector of range entries mapping child bus addresses to parent bus addresses.
    pub fn ranges(&self, parent_address_cells: u32) -> Option<Vec<RangesEntry>> {
        let prop = self.get_property("ranges")?;
        let mut entries = Vec::new();
        let mut reader = prop.as_reader();

        // Current node's #address-cells for child node addresses
        let child_address_cells = self.address_cells().unwrap_or(2) as usize;
        // Parent node's #address-cells for parent bus addresses
        let parent_addr_cells = parent_address_cells as usize;
        // Current node's #size-cells
        let size_cells = self.size_cells().unwrap_or(1) as usize;

        while let (Some(child_addr), Some(parent_addr), Some(size)) = (
            reader.read_cells(child_address_cells),
            reader.read_cells(parent_addr_cells),
            reader.read_cells(size_cells),
        ) {
            entries.push(RangesEntry {
                child_bus_address: child_addr,
                parent_bus_address: parent_addr,
                length: size,
            });
        }

        Some(entries)
    }

    /// Rebuilds the name cache from the current children list.
    fn rebuild_name_cache(&mut self) {
        self.name_cache.clear();
        for (idx, child) in self.children.iter().enumerate() {
            self.name_cache.insert(child.name.clone(), idx);
        }
    }

    /// Returns the `compatible` property as a string iterator.
    pub fn compatible(&self) -> Option<impl Iterator<Item = &str>> {
        let prop = self.get_property("compatible")?;
        Some(prop.as_str_iter())
    }

    /// Returns an iterator over all compatible strings.
    pub fn compatibles(&self) -> impl Iterator<Item = &str> {
        self.get_property("compatible")
            .map(|prop| prop.as_str_iter())
            .into_iter()
            .flatten()
    }

    /// Returns the `device_type` property value.
    pub fn device_type(&self) -> Option<&str> {
        let prop = self.get_property("device_type")?;
        prop.as_str()
    }

    /// Removes a child node and its subtree by exact path.
    ///
    /// Only supports exact path matching, not wildcard matching.
    ///
    /// # Arguments
    ///
    /// * `path` - The removal path, format: "soc/gpio@1000" or "/soc/gpio@1000"
    ///
    /// # Returns
    ///
    /// * `Ok(Option<Node>)` - The removed node if found, None if path doesn't exist
    /// * `Err(FdtError)` - If the path format is invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// # use fdt_edit::Node;
    /// let mut root = Node::new("");
    /// // Add test nodes
    /// let mut soc = Node::new("soc");
    /// soc.add_child(Node::new("gpio@1000"));
    /// root.add_child(soc);
    ///
    /// // Remove node by exact path
    /// let removed = root.remove_by_path("soc/gpio@1000")?;
    /// assert!(removed.is_some());
    /// # Ok::<(), fdt_raw::FdtError>(())
    /// ```
    pub fn remove_by_path(&mut self, path: &str) -> Result<Option<Node>, fdt_raw::FdtError> {
        let normalized_path = path.trim_start_matches('/');
        if normalized_path.is_empty() {
            return Err(fdt_raw::FdtError::InvalidInput);
        }

        let parts: Vec<&str> = normalized_path.split('/').collect();
        if parts.is_empty() {
            return Err(fdt_raw::FdtError::InvalidInput);
        }
        if parts.len() == 1 {
            // Remove direct child (exact match)
            let child_name = parts[0];
            Ok(self.remove_child(child_name))
        } else {
            // Need to recurse to parent node for removal
            self.remove_child_recursive(&parts, 0)
        }
    }

    /// Recursive implementation for removing child nodes.
    ///
    /// Finds the parent of the node to remove, then removes the target child
    /// from that parent node.
    fn remove_child_recursive(
        &mut self,
        parts: &[&str],
        index: usize,
    ) -> Result<Option<Node>, fdt_raw::FdtError> {
        if index >= parts.len() - 1 {
            // Already at the parent level of the node to remove
            let child_name_to_remove = parts[index];
            Ok(self.remove_child(child_name_to_remove))
        } else {
            // Continue recursing down
            let current_part = parts[index];

            // Intermediate levels only support exact matching (using cache)
            if let Some(&child_index) = self.name_cache.get(current_part) {
                self.children[child_index].remove_child_recursive(parts, index + 1)
            } else {
                // Path doesn't exist
                Ok(None)
            }
        }
    }
}

impl From<&fdt_raw::Node<'_>> for Node {
    fn from(raw: &fdt_raw::Node<'_>) -> Self {
        let mut new_node = Node::new(raw.name());
        // Copy properties
        for raw_prop in raw.properties() {
            let prop = Property::from(&raw_prop);
            new_node.set_property(prop);
        }
        new_node
    }
}
