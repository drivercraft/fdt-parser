//! Main Flattened Device Tree (FDT) parser.
//!
//! This module provides the primary `Fdt` type that represents a parsed
//! device tree blob. It offers methods for traversing nodes, resolving
//! paths, translating addresses, and accessing special nodes like
//! /chosen and /memory.

use core::fmt;

use crate::{
    Chosen, FdtError, Memory, MemoryReservation, Node, Property, VecRange, data, data::Bytes,
    fmt_utils, header::Header, iter::FdtIter,
};

/// Iterator over memory reservation entries.
///
/// The memory reservation block contains a list of physical memory regions
/// that must be preserved during boot. This iterator yields each reservation
/// entry until it reaches the terminating entry (address=0, size=0).
pub struct MemoryReservationIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for MemoryReservationIter<'a> {
    type Item = MemoryReservation;

    fn next(&mut self) -> Option<Self::Item> {
        // Ensure we have enough data to read a complete entry
        if self.offset + data::MEM_RSV_ENTRY_SIZE > self.data.len() {
            return None;
        }

        // Read address (8 bytes, big-endian)
        let address_bytes = &self.data[self.offset..self.offset + 8];
        let address = u64::from_be_bytes(address_bytes.try_into().unwrap());
        self.offset += 8;

        // Read size (8 bytes, big-endian)
        let size_bytes = &self.data[self.offset..self.offset + 8];
        let size = u64::from_be_bytes(size_bytes.try_into().unwrap());
        self.offset += 8;

        // Check for terminator (both address and size are zero)
        if address == 0 && size == 0 {
            return None;
        }

        Some(MemoryReservation { address, size })
    }
}

/// A parsed Flattened Device Tree (FDT).
///
/// This is the main type for working with device tree blobs. It provides
/// methods for traversing the tree, finding nodes by path, translating
/// addresses, and accessing special nodes like /chosen and /memory.
///
/// The `Fdt` holds a reference to the underlying device tree data and
/// performs zero-copy parsing where possible.
#[derive(Clone)]
pub struct Fdt<'a> {
    header: Header,
    pub(crate) data: Bytes<'a>,
}

impl<'a> Fdt<'a> {
    /// Create a new `Fdt` from a byte slice.
    ///
    /// Parses the FDT header and validates the magic number. The slice
    /// must contain a complete, valid device tree blob.
    ///
    /// # Errors
    ///
    /// Returns `FdtError` if the header is invalid, the magic number
    /// doesn't match, or the buffer is too small.
    pub fn from_bytes(data: &'a [u8]) -> Result<Fdt<'a>, FdtError> {
        let header = Header::from_bytes(data)?;
        if data.len() < header.totalsize as usize {
            return Err(FdtError::BufferTooSmall {
                pos: header.totalsize as usize,
            });
        }
        let buffer = Bytes::new(data);

        Ok(Fdt {
            header,
            data: buffer,
        })
    }

    /// Create a new `Fdt` from a raw pointer.
    ///
    /// Parses an FDT from the memory location pointed to by `ptr`.
    /// This is useful when working with device trees loaded by bootloaders.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// memory region of at least `totalsize` bytes that contains a valid
    /// device tree blob. The memory must remain valid for the lifetime `'a`.
    ///
    /// # Errors
    ///
    /// Returns `FdtError` if the header is invalid or the magic number
    /// doesn't match.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Fdt<'a>, FdtError> {
        let header = unsafe { Header::from_ptr(ptr)? };

        let data_slice = unsafe { core::slice::from_raw_parts(ptr, header.totalsize as _) };
        let data = Bytes::new(data_slice);

        Ok(Fdt { header, data })
    }

    /// Returns a reference to the FDT header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Returns the underlying byte slice.
    pub fn as_slice(&self) -> &'a [u8] {
        self.data.as_slice()
    }

    /// Returns an iterator over all nodes in the device tree.
    pub fn all_nodes(&self) -> FdtIter<'a> {
        FdtIter::new(self.clone())
    }

    /// Find a node by its absolute path or alias.
    ///
    /// The path can be an absolute path starting with '/', or an alias
    /// defined in the /aliases node. Returns `None` if the node is not found.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let node = fdt.find_by_path("/soc@30000000/serial@10000");
    /// let uart = fdt.find_by_path("serial0");  // Using alias
    /// ```
    pub fn find_by_path(&self, path: &str) -> Option<Node<'a>> {
        let path = self.normalize_path(path)?;
        let split = path.trim_matches('/').split('/');

        let mut current_iter = self.all_nodes();
        let mut found_node: Option<Node<'a>> = None;

        for part in split {
            let mut found = false;
            for node in current_iter.by_ref() {
                let node_name = node.name();
                if node_name == part {
                    found = true;
                    found_node = Some(node);
                    break;
                }
            }
            if !found {
                return None;
            }
        }

        found_node
    }

    /// Find all direct children of a node at the given path.
    ///
    /// Returns an iterator over all direct child nodes (one level deeper)
    /// of the node at the specified path. Returns `None` if the node is
    /// not found.
    ///
    /// Only direct children are yielded — grandchildren and deeper
    /// descendants are skipped.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // List all direct children of /soc
    /// if let Some(children) = fdt.find_children_by_path("/soc") {
    ///     for child in children {
    ///         println!("{}", child.name());
    ///     }
    /// }
    /// ```
    pub fn find_children_by_path(&self, path: &str) -> Option<impl Iterator<Item = Node<'a>> + 'a> {
        let path = self.normalize_path(path)?;
        let split = path.trim_matches('/').split('/');

        let mut iter = self.all_nodes();
        let mut target_level = 0usize;

        for part in split {
            if part.is_empty() {
                // Root path "/" — skip the root node itself
                iter.next();
                break;
            }
            let mut found = false;
            for node in iter.by_ref() {
                if node.name() == part {
                    found = true;
                    target_level = node.level();
                    break;
                }
            }
            if !found {
                return None;
            }
        }

        let child_level = target_level + 1;
        Some(ChildrenIter {
            node_iter: iter,
            child_level,
            done: false,
        })
    }

    /// Resolve an alias to its full path.
    ///
    /// Looks up the alias in the /aliases node and returns the corresponding
    /// path string.
    fn resolve_alias(&self, alias: &str) -> Option<&'a str> {
        let aliases_node = self.find_by_path("/aliases")?;
        aliases_node.find_property_str(alias)
    }

    /// Normalize a path to an absolute path.
    ///
    /// If the path starts with '/', it's returned as-is. Otherwise,
    /// it's treated as an alias and resolved.
    fn normalize_path(&self, path: &'a str) -> Option<&'a str> {
        if path.starts_with('/') {
            Some(path)
        } else {
            self.resolve_alias(path)
        }
    }

    /// Translate a device address to a CPU physical address.
    ///
    /// This function implements address translation similar to Linux's
    /// `of_translate_address`. It walks up the device tree hierarchy,
    /// applying each parent's `ranges` property to translate the child
    /// address space to the parent address space.
    ///
    /// The translation starts from the node at `path` and walks up through
    /// each parent, applying the `ranges` property until reaching the root.
    ///
    /// # Arguments
    ///
    /// * `path` - Node path (absolute path starting with '/' or alias name)
    /// * `address` - Device address from the node's `reg` property
    ///
    /// # Returns
    ///
    /// The translated CPU physical address. If translation fails at any
    /// point (e.g., a parent node has no `ranges` property), the original
    /// address is returned.
    pub fn translate_address(&self, path: &'a str, address: u64) -> u64 {
        let mut addresses = [address];
        self.translate_addresses(path, &mut addresses);
        addresses[0]
    }

    /// Translate multiple device addresses to CPU physical addresses in a single pass.
    ///
    /// This is more efficient than calling `translate_address` multiple times
    /// for the same node path, because the tree is walked only once. Each
    /// parent node's `ranges` property is looked up once and applied to all
    /// addresses in the batch.
    ///
    /// # Arguments
    ///
    /// * `path` - Node path (absolute path starting with '/' or alias name)
    /// * `addresses` - Mutable slice of device addresses to translate in place.
    ///   The addresses are modified with the translated CPU physical addresses.
    ///
    /// If translation fails for any address at any level, the original address
    /// value is preserved for that address.
    pub fn translate_addresses(&self, path: &'a str, addresses: &mut [u64]) {
        let path = match self.normalize_path(path) {
            Some(p) => p,
            None => return,
        };

        let path_parts = Self::split_path(path);
        if path_parts.is_empty() {
            return;
        }

        self.translate_addresses_with_parts(&path_parts, addresses);
    }

    /// Splits an absolute path into its component parts.
    ///
    /// Takes a path like "/soc/serial@0" and returns ["soc", "serial@0"].
    fn split_path(path: &str) -> heapless::Vec<&str, 16> {
        path.trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Performs batch address translation using pre-split path components.
    ///
    /// Walks up the tree from the deepest node, applying `ranges` at each level
    /// to all addresses in the slice. Each parent node is looked up only once.
    fn translate_addresses_with_parts(&self, path_parts: &[&str], addresses: &mut [u64]) {
        // Walk up from the deepest node, applying ranges at each level
        // We start from the second-to-last level (the target node itself is skipped)
        for depth in (0..path_parts.len()).rev() {
            let parent_parts = &path_parts[..depth];

            if parent_parts.is_empty() {
                // Reached root node, no more translation needed
                break;
            }

            if let Some(parent_node) = self.find_node_by_parts(parent_parts) {
                let ranges = match parent_node.ranges() {
                    Some(r) => r,
                    None => break, // No ranges property, stop translation
                };

                // Apply ranges to all addresses in batch
                for addr in addresses.iter_mut() {
                    *addr = Self::apply_ranges_one(&ranges, *addr);
                }
            }
        }
    }

    /// Finds a node by its path component parts.
    fn find_node_by_parts(&self, parts: &[&str]) -> Option<Node<'a>> {
        let mut path = heapless::String::<256>::new();
        path.push('/').ok();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                path.push('/').ok();
            }
            path.push_str(part).ok();
        }
        self.find_by_path(path.as_str())
    }

    /// Translates a single address using the given ranges.
    ///
    /// If the address falls within a range, it is translated. Otherwise,
    /// the original address is returned unchanged.
    fn apply_ranges_one(ranges: &VecRange<'_>, address: u64) -> u64 {
        for range in ranges.iter() {
            // Check if the address falls within this range
            if address >= range.child_address && address < range.child_address + range.length {
                let offset = address - range.child_address;
                return range.parent_address + offset;
            }
        }

        // No matching range found, return as-is
        address
    }

    /// Returns an iterator over memory reservation entries.
    pub fn memory_reservations(&self) -> MemoryReservationIter<'a> {
        MemoryReservationIter {
            data: self.data.as_slice(),
            offset: self.header.off_mem_rsvmap as usize,
        }
    }

    /// Returns the /chosen node if it exists.
    pub fn chosen(&self) -> Option<Chosen<'a>> {
        for node in self.all_nodes() {
            if let Node::Chosen(c) = node {
                return Some(c);
            }
        }
        None
    }

    /// Returns an iterator over all memory nodes.
    pub fn memory(&self) -> impl Iterator<Item = Memory<'a>> + 'a {
        self.all_nodes().filter_map(|node| {
            if let Node::Memory(mem) = node {
                Some(mem)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over nodes in the /reserved-memory region.
    pub fn reserved_memory(&self) -> impl Iterator<Item = Node<'a>> + 'a {
        ReservedMemoryIter {
            node_iter: self.all_nodes(),
            in_reserved_memory: false,
            reserved_level: 0,
        }
    }
}

/// Iterator over nodes in the /reserved-memory region.
///
/// Yields all child nodes of the /reserved-memory node, which describe
/// memory regions that are reserved for specific purposes.
struct ReservedMemoryIter<'a> {
    node_iter: FdtIter<'a>,
    in_reserved_memory: bool,
    reserved_level: usize,
}

impl<'a> Iterator for ReservedMemoryIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for node in self.node_iter.by_ref() {
            if node.name() == "reserved-memory" {
                self.in_reserved_memory = true;
                self.reserved_level = node.level();
                continue;
            }

            if self.in_reserved_memory {
                if node.level() <= self.reserved_level {
                    // Left the reserved-memory node
                    self.in_reserved_memory = false;
                    return None;
                } else {
                    return Some(node);
                }
            }
        }
        None
    }
}

/// Iterator over direct children of a specific node.
///
/// Yields only nodes whose level equals `child_level`. Nodes deeper
/// than `child_level` (grandchildren) are skipped, and iteration stops
/// when leaving the parent's subtree (level < child_level).
struct ChildrenIter<'a> {
    node_iter: FdtIter<'a>,
    child_level: usize,
    done: bool,
}

impl<'a> Iterator for ChildrenIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        for node in self.node_iter.by_ref() {
            if node.level() == self.child_level {
                return Some(node);
            }
            if node.level() < self.child_level {
                // Left the parent's subtree
                self.done = true;
                return None;
            }
            // node.level() > self.child_level: grandchild, skip
        }
        None
    }
}

impl fmt::Display for Fdt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "/dts-v1/;")?;
        writeln!(f)?;

        let mut state = DisplayState::new();

        for node in self.all_nodes() {
            self.close_open_nodes(f, &mut state, node.level())?;
            self.write_node(f, &node)?;
            state.prev_level = node.level() + 1;
        }

        self.close_all_nodes(f, &mut state)
    }
}

/// State for tracking the display output during tree traversal.
struct DisplayState {
    prev_level: usize,
}

impl DisplayState {
    fn new() -> Self {
        Self { prev_level: 0 }
    }
}

impl Fdt<'_> {
    /// Writes a single node in DTS format.
    fn write_node(&self, f: &mut fmt::Formatter<'_>, node: &Node<'_>) -> fmt::Result {
        fmt_utils::write_indent(f, node.level(), "    ")?;
        let name = Self::format_node_name(node.name());
        writeln!(f, "{} {{", name)?;

        for prop in node.properties() {
            fmt_utils::write_indent(f, node.level() + 1, "    ")?;
            writeln!(f, "{};", prop)?;
        }
        Ok(())
    }

    /// Formats a node name, replacing empty names with "/".
    fn format_node_name(name: &str) -> &str {
        if name.is_empty() { "/" } else { name }
    }

    /// Closes nodes that are no longer open in the tree traversal.
    fn close_open_nodes(
        &self,
        f: &mut fmt::Formatter<'_>,
        state: &mut DisplayState,
        current_level: usize,
    ) -> fmt::Result {
        while state.prev_level > current_level {
            state.prev_level -= 1;
            fmt_utils::write_indent(f, state.prev_level, "    ")?;
            writeln!(f, "}};\n")?;
        }
        Ok(())
    }

    /// Closes all remaining open nodes at the end of output.
    fn close_all_nodes(&self, f: &mut fmt::Formatter<'_>, state: &mut DisplayState) -> fmt::Result {
        while state.prev_level > 0 {
            state.prev_level -= 1;
            fmt_utils::write_indent(f, state.prev_level, "    ")?;
            writeln!(f, "}};\n")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Fdt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Fdt {{")?;
        writeln!(f, "\theader: {:?}", self.header)?;
        writeln!(f, "\tnodes:")?;

        for node in self.all_nodes() {
            self.debug_node(f, &node)?;
        }

        writeln!(f, "}}")
    }
}

impl Fdt<'_> {
    /// Writes a node in debug format.
    fn debug_node(&self, f: &mut fmt::Formatter<'_>, node: &Node<'_>) -> fmt::Result {
        let level = node.level();
        fmt_utils::write_indent(f, level + 2, "\t")?;

        let name = Self::format_node_name(node.name());
        writeln!(
            f,
            "[{}] address_cells={}, size_cells={}",
            name, node.address_cells, node.size_cells
        )?;

        for prop in node.properties() {
            self.debug_property(f, level, &prop)?;
        }
        Ok(())
    }

    /// Writes a property in debug format.
    fn debug_property(
        &self,
        f: &mut fmt::Formatter<'_>,
        level: usize,
        prop: &Property<'_>,
    ) -> fmt::Result {
        fmt_utils::write_indent(f, level + 3, "\t")?;

        match () {
            () if prop.as_address_cells().is_some() => {
                writeln!(f, "#address-cells: {}", prop.as_address_cells().unwrap())?
            }
            () if prop.as_size_cells().is_some() => {
                writeln!(f, "#size-cells: {}", prop.as_size_cells().unwrap())?
            }
            () if prop.as_interrupt_cells().is_some() => writeln!(
                f,
                "#interrupt-cells: {}",
                prop.as_interrupt_cells().unwrap()
            )?,
            () if prop.as_status().is_some() => {
                writeln!(f, "status: {:?}", prop.as_status().unwrap())?
            }
            () if prop.as_phandle().is_some() => {
                writeln!(f, "phandle: {}", prop.as_phandle().unwrap())?
            }
            () if prop.is_empty() => writeln!(f, "{}", prop.name())?,
            () if prop.as_str().is_some() => {
                writeln!(f, "{}: \"{}\"", prop.name(), prop.as_str().unwrap())?
            }
            () if prop.len() == 4 => {
                let v = u32::from_be_bytes(prop.data().as_slice().try_into().unwrap());
                writeln!(f, "{}: {:#x}", prop.name(), v)?
            }
            () => writeln!(f, "{}: <{} bytes>", prop.name(), prop.len())?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;

    #[test]
    fn test_memory_reservation_iterator() {
        // Create simple test data: one memory reservation entry + terminator
        let mut test_data = [0u8; data::MEM_RSV_ENTRY_SIZE * 2];

        // Address: 0x80000000, Size: 0x10000000 (256MB)
        test_data[0..8].copy_from_slice(&0x80000000u64.to_be_bytes());
        test_data[8..16].copy_from_slice(&0x10000000u64.to_be_bytes());
        // Terminator: address=0, size=0
        test_data[16..24].copy_from_slice(&0u64.to_be_bytes());
        test_data[24..32].copy_from_slice(&0u64.to_be_bytes());

        let iter = MemoryReservationIter {
            data: &test_data,
            offset: 0,
        };

        let reservations: Vec<MemoryReservation, 4> = iter.collect();
        assert_eq!(reservations.len(), 1);
        assert_eq!(reservations[0].address, 0x80000000);
        assert_eq!(reservations[0].size, 0x10000000);
    }

    #[test]
    fn test_empty_memory_reservation_iterator() {
        // Only terminator
        let mut test_data = [0u8; data::MEM_RSV_ENTRY_SIZE];
        test_data[0..8].copy_from_slice(&0u64.to_be_bytes());
        test_data[8..16].copy_from_slice(&0u64.to_be_bytes());

        let iter = MemoryReservationIter {
            data: &test_data,
            offset: 0,
        };

        let reservations: Vec<MemoryReservation, 4> = iter.collect();
        assert_eq!(reservations.len(), 0);
    }
}
