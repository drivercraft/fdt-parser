//! Main Flattened Device Tree (FDT) parser.
//!
//! This module provides the primary `Fdt` type that represents a parsed
//! device tree blob. It offers methods for traversing nodes, resolving
//! paths, translating addresses, and accessing special nodes like
//! /chosen and /memory.

use core::fmt;

use crate::{
    Chosen, FdtError, Memory, MemoryReservation, Node, data::Bytes, header::Header, iter::FdtIter,
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
        // Ensure we have enough data to read address and size (8 bytes each)
        if self.offset + 16 > self.data.len() {
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

/// Helper function for writing indentation during formatting.
fn write_indent(f: &mut fmt::Formatter<'_>, count: usize, ch: &str) -> fmt::Result {
    for _ in 0..count {
        write!(f, "{}", ch)?;
    }
    Ok(())
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
        let path = match self.normalize_path(path) {
            Some(p) => p,
            None => return address,
        };

        // Split path into component parts
        let path_parts: heapless::Vec<&str, 16> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if path_parts.is_empty() {
            return address;
        }

        let mut current_address = address;

        // Walk up from the deepest node, applying ranges at each level
        // Note: We start from the second-to-last level (the target node itself is skipped)
        for depth in (0..path_parts.len()).rev() {
            // Build the path to the current parent level
            let parent_parts = &path_parts[..depth];
            if parent_parts.is_empty() {
                // Reached root node, no more translation needed
                break;
            }

            // Find the parent node
            let mut parent_path = heapless::String::<256>::new();
            parent_path.push('/').ok();
            for (i, part) in parent_parts.iter().enumerate() {
                if i > 0 {
                    parent_path.push('/').ok();
                }
                parent_path.push_str(part).ok();
            }

            let parent_node = match self.find_by_path(parent_path.as_str()) {
                Some(node) => node,
                None => continue,
            };

            // Get the parent's ranges property
            let ranges = match parent_node.ranges() {
                Some(r) => r,
                None => {
                    // No ranges property, stop translation
                    break;
                }
            };

            // Look for a matching translation rule in ranges
            let mut found = false;
            for range in ranges.iter() {
                // Check if the address falls within this range
                if current_address >= range.child_address
                    && current_address < range.child_address + range.length
                {
                    // Calculate offset in child address space
                    let offset = current_address - range.child_address;
                    // Translate to parent address space
                    current_address = range.parent_address + offset;
                    found = true;
                    break;
                }
            }

            if !found {
                // No matching range found, keep current address and continue
                // This typically means translation failed, but we try upper levels
            }
        }

        current_address
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

impl fmt::Display for Fdt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "/dts-v1/;")?;
        writeln!(f)?;

        let mut prev_level = 0;

        for node in self.all_nodes() {
            let level = node.level();

            // Close nodes from the previous level
            while prev_level > level {
                prev_level -= 1;
                write_indent(f, prev_level, "    ")?;
                writeln!(f, "}};\n")?;
            }

            write_indent(f, level, "    ")?;
            let name = if node.name().is_empty() {
                "/"
            } else {
                node.name()
            };

            // Print node header
            writeln!(f, "{} {{", name)?;

            // Print properties
            for prop in node.properties() {
                write_indent(f, level + 1, "    ")?;
                writeln!(f, "{};", prop)?;
            }

            prev_level = level + 1;
        }

        // Close remaining nodes
        while prev_level > 0 {
            prev_level -= 1;
            write_indent(f, prev_level, "    ")?;
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
            let level = node.level();
            // Base indentation is 2 tabs, plus 1 tab per level
            write_indent(f, level + 2, "\t")?;

            let name = if node.name().is_empty() {
                "/"
            } else {
                node.name()
            };

            // Print node name and basic info
            writeln!(
                f,
                "[{}] address_cells={}, size_cells={}",
                name, node.address_cells, node.size_cells
            )?;

            // Print properties
            for prop in node.properties() {
                write_indent(f, level + 3, "\t")?;
                if let Some(v) = prop.as_address_cells() {
                    writeln!(f, "#address-cells: {}", v)?;
                } else if let Some(v) = prop.as_size_cells() {
                    writeln!(f, "#size-cells: {}", v)?;
                } else if let Some(v) = prop.as_interrupt_cells() {
                    writeln!(f, "#interrupt-cells: {}", v)?;
                } else if let Some(s) = prop.as_status() {
                    writeln!(f, "status: {:?}", s)?;
                } else if let Some(p) = prop.as_phandle() {
                    writeln!(f, "phandle: {}", p)?;
                } else {
                    // Default handling for unknown properties
                    if prop.is_empty() {
                        writeln!(f, "{}", prop.name())?;
                    } else if let Some(s) = prop.as_str() {
                        writeln!(f, "{}: \"{}\"", prop.name(), s)?;
                    } else if prop.len() == 4 {
                        let v = u32::from_be_bytes(prop.data().as_slice().try_into().unwrap());
                        writeln!(f, "{}: {:#x}", prop.name(), v)?;
                    } else {
                        writeln!(f, "{}: <{} bytes>", prop.name(), prop.len())?;
                    }
                }
            }
        }

        writeln!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;

    #[test]
    fn test_memory_reservation_iterator() {
        // Create simple test data: one memory reservation entry + terminator
        let mut test_data = [0u8; 32];

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
        let mut test_data = [0u8; 16];
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
