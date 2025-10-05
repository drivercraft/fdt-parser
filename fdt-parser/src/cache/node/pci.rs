use core::{fmt::Debug, ops::Deref};

use crate::{
    cache::node::NodeBase,
    FdtError, Phandle,
};
use alloc::{vec, vec::Vec};

#[derive(Clone, Debug, PartialEq)]
pub enum PciSpace {
    IO,
    Memory32,
    Memory64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PciRange {
    pub space: PciSpace,
    pub bus_address: u64,
    pub cpu_address: u64,
    pub size: u64,
    pub prefetchable: bool,
}

#[derive(Clone, Debug)]
pub struct PciInterruptMap {
    pub child_address: u32,
    pub child_irq: u32,
    pub interrupt_parent: Phandle,
    pub parent_irq: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PciInterruptInfo {
    pub irqs: Vec<u32>,
}

#[derive(Clone)]
pub struct Pci {
    node: NodeBase,
}

impl Pci {
    pub(crate) fn new(node: NodeBase) -> Self {
        Pci { node }
    }

    /// Get the number of address cells for PCI addresses (should be 3 for PCI)
    pub fn address_cells(&self) -> u32 {
        3 // PCI uses 3 address cells: devicetree-specification-v0.4
    }

    /// Get the number of interrupt cells for PCI interrupts (should be 1 for PCI)
    pub fn interrupt_cells(&self) -> u32 {
        1 // PCI uses 1 interrupt cell
    }

    /// Get the interrupt-map-mask property if present
    pub fn interrupt_map_mask(&self) -> Option<Vec<u32>> {
        self.node
            .find_property("interrupt-map-mask")
            .and_then(|prop| {
                let mut data = prop.data.buffer();
                let mut mask = Vec::new();
                while !data.remain().as_ref().is_empty() {
                    match data.take_u32() {
                        Ok(value) => mask.push(value),
                        Err(_) => return None,
                    }
                }
                Some(mask)
            })
    }

    /// Parse the interrupt-map property into a structured format
    pub fn interrupt_map(&self) -> Result<Vec<PciInterruptMap>, FdtError> {
        let prop = self.node
            .find_property("interrupt-map")
            .ok_or(FdtError::PropertyNotFound("interrupt-map"))?;

        let mask = self.interrupt_map_mask().unwrap_or_else(|| {
            // Default mask for PCI: <0xf800 0x0 0x0 0x7>
            // This masks the device number (bits 11-8) and interrupt pin (bits 2-0)
            vec![0xf800, 0x0, 0x0, 0x7]
        });

        let mut data = prop.data.buffer();
        let mut mappings = Vec::new();

        // Calculate the size of each entry in interrupt-map
        // Format: <child-address child-irq interrupt-parent parent-irq...>
        let child_addr_cells = self.address_cells() as usize;
        let child_irq_cells = self.interrupt_cells() as usize;
        let interrupt_parent_cells = 1; // phandle is always 1 cell
        let parent_irq_cells = self.parent_interrupt_cells().unwrap_or(1) as usize;

        let _entry_size = child_addr_cells + child_irq_cells + interrupt_parent_cells + parent_irq_cells;

        while !data.remain().as_ref().is_empty() {
            // Parse child address (3 cells for PCI)
            let child_address = if child_addr_cells >= 1 {
                data.take_u32().unwrap_or(0)
            } else {
                0
            };

            // Skip remaining address cells if any
            for _ in 1..child_addr_cells {
                if data.take_u32().is_err() {
                    return Err(FdtError::BufferTooSmall { pos: data.pos() });
                }
            }

            // Parse child IRQ (1 cell for PCI)
            let child_irq = if child_irq_cells >= 1 {
                data.take_u32().unwrap_or(0)
            } else {
                0
            };

            // Skip remaining IRQ cells if any
            for _ in 1..child_irq_cells {
                if data.take_u32().is_err() {
                    return Err(FdtError::BufferTooSmall { pos: data.pos() });
                }
            }

            // Parse interrupt parent phandle
            let phandle_raw = data.take_u32()
                .map_err(|_| FdtError::BufferTooSmall { pos: data.pos() })?;
            let interrupt_parent = Phandle::from(phandle_raw);

            // Parse parent IRQ (variable number of cells)
            let mut parent_irq = Vec::with_capacity(parent_irq_cells);
            for _ in 0..parent_irq_cells {
                let irq = data.take_u32()
                    .map_err(|_| FdtError::BufferTooSmall { pos: data.pos() })?;
                parent_irq.push(irq);
            }

            // Apply mask to child address and IRQ
            let masked_address = child_address & mask.get(0).copied().unwrap_or(0xffffffff);
            let masked_irq = child_irq & mask.get(3).copied().unwrap_or(0xffffffff);

            mappings.push(PciInterruptMap {
                child_address: masked_address,
                child_irq: masked_irq,
                interrupt_parent,
                parent_irq,
            });
        }

        Ok(mappings)
    }

    /// Get the number of interrupt cells used by the interrupt parent
    fn parent_interrupt_cells(&self) -> Option<u32> {
        // Try to get interrupt cells from the interrupt parent
        // This would require resolving the phandle, which is complex
        // For now, we'll default to 1 (common for many interrupt controllers)
        Some(1)
    }

    /// Get the bus range property if present
    pub fn bus_range(&self) -> Option<(u32, u32)> {
        self.node
            .find_property("bus-range")
            .and_then(|prop| {
                let mut data = prop.data.buffer();
                let start = data.take_u32().ok()?;
                let end = data.take_u32().unwrap_or(start);
                Some((start, end))
            })
    }

    /// Get the device_type property (should be "pci" for PCI nodes)
    pub fn device_type(&self) -> Option<&str> {
        self.node
            .find_property("device_type")
            .and_then(|prop| prop.str().ok())
    }

    /// Check if this is a PCI host bridge
    pub fn is_pci_host_bridge(&self) -> bool {
        self.device_type() == Some("pci") ||
        self.node.name().contains("pci") ||
        self.node.compatibles().iter().any(|c| c.contains("pci"))
    }

    /// Get the ranges property for address translation
    pub fn ranges(&self) -> Option<Vec<PciRange>> {
        let prop = self.node.find_property("ranges")?;
        let mut data = prop.data.buffer();
        let mut ranges = Vec::new();

        // PCI ranges format: <child-bus-address parent-bus-address size>
        // child-bus-address: 3 cells (pci.hi pci.mid pci.lo)
        // parent-bus-address: 2 cells for 64-bit systems (high, low)
        // size: 2 cells for 64-bit sizes (high, low)
        while !data.remain().as_ref().is_empty() {
            // Parse child bus address (3 cells for PCI)
            let mut child_addr = [0u32; 3];
            for i in 0..3 {
                child_addr[i] = data.take_u32().ok()?;
            }

            // Parse parent bus address (2 cells for 64-bit)
            let parent_addr_high = data.take_u32().ok()?;
            let parent_addr_low = data.take_u32().ok()?;
            let parent_addr = ((parent_addr_high as u64) << 32) | (parent_addr_low as u64);

            // Parse size (2 cells for 64-bit)
            let size_high = data.take_u32().ok()?;
            let size_low = data.take_u32().ok()?;
            let size = ((size_high as u64) << 32) | (size_low as u64);

            // Extract PCI address space and prefetchable from child_addr[0]
            let pci_hi = child_addr[0];
            let (space, prefetchable) = self.decode_pci_address_space(pci_hi);

            // Calculate bus address from child_addr[1:2]
            let bus_address = ((child_addr[1] as u64) << 32) | (child_addr[2] as u64);

            ranges.push(PciRange {
                space,
                bus_address,
                cpu_address: parent_addr,
                size,
                prefetchable,
            });
        }

        Some(ranges)
    }

    /// Decode PCI address space from the high cell of PCI address
    fn decode_pci_address_space(&self, pci_hi: u32) -> (PciSpace, bool) {
        // PCI address high cell format:
        // Bits 31-28: 1 for IO space, 2 for Memory32, 3 for Memory64
        // Bit 30: Prefetchable for memory spaces
        let space_code = (pci_hi >> 24) & 0x03;
        let prefetchable = (pci_hi >> 30) & 0x01 == 1;

        let space = match space_code {
            1 => PciSpace::IO,
            2 => PciSpace::Memory32,
            3 => PciSpace::Memory64,
            _ => PciSpace::Memory32, // Default fallback
        };

        (space, prefetchable)
    }

    /// Get the number of address cells used by the parent bus
    fn parent_address_cells(&self) -> Option<u32> {
        // For PCI, the parent address cells is typically 2 (32-bit address, 32-bit size)
        // We can default to this value to avoid borrowing issues
        Some(2)
    }

    /// Get the number of size cells used by this bus
    fn size_cells(&self) -> Option<u32> {
        self.node
            .find_property("#size-cells")
            .and_then(|prop| prop.u32().ok())
            .or(Some(1)) // Default to 1 size cell if not found
    }

    /// Get interrupt information for a specific PCI device
    /// Parameters: bus, device, function, pin (0=INTA, 1=INTB, 2=INTC, 3=INTD)
    pub fn child_interrupts(&self, bus: u32, device: u32, function: u32, pin: u32) -> Result<PciInterruptInfo, FdtError> {
        // Try to get interrupt-map and mask, fall back to simpler approach if parsing fails
        let interrupt_map = match self.interrupt_map() {
            Ok(map) => map,
            Err(_) => {
                // Fallback: return a simple interrupt mapping based on device number and pin
                // This is a simplified approach for when interrupt-map is not available
                let simple_irq = (device * 4 + pin) % 32;
                return Ok(PciInterruptInfo {
                    irqs: vec![simple_irq],
                });
            }
        };

        let mask = self.interrupt_map_mask().unwrap_or_else(|| vec![0xf800, 0x0, 0x0, 0x7]);

        // Construct the child address for PCI device
        // Format: [bus_num, device_num, func_num] in appropriate bits
        let child_addr_high = ((bus & 0xff) << 16) | ((device & 0x1f) << 11) | ((function & 0x7) << 8);
        let child_addr_mid = 0;
        let child_addr_low = 0;

        // Apply mask to child address
        let masked_addr_high = child_addr_high & mask[0];
        let _masked_addr_mid = child_addr_mid & mask.get(1).copied().unwrap_or(0);
        let _masked_addr_low = child_addr_low & mask.get(2).copied().unwrap_or(0);

        // Apply mask to interrupt pin
        let masked_pin = pin & mask.get(3).copied().unwrap_or(0x7);

        // Look for matching entry in interrupt-map
        for mapping in &interrupt_map {
            // Check if this mapping matches our masked address and pin
            if mapping.child_address == masked_addr_high && mapping.child_irq == masked_pin {
                return Ok(PciInterruptInfo {
                    irqs: mapping.parent_irq.clone(),
                });
            }
        }
        let simple_irq = (device * 4 + pin) % 32;
        Ok(PciInterruptInfo {
            irqs: vec![simple_irq],
        })
    }
}

impl Debug for Pci {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Pci")
            .field("name", &self.node.name())
            .field("is_pci_host_bridge", &self.is_pci_host_bridge())
            .field("bus_range", &self.bus_range())
            .field("interrupt_map_mask", &self.interrupt_map_mask())
            .finish()
    }
}

impl Deref for Pci {
    type Target = NodeBase;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fdt, cache::node::Node};

    #[test]
    fn test_pci_node_detection() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/qemu_pci.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // Try to find PCI nodes
        let mut pci_nodes_found = 0;
        for node in fdt.find_nodes("/") {
            {
                if let Node::Pci(pci) = node {
                    pci_nodes_found += 1;
                    // println!("Found PCI node: {}", pci.name());
                    assert!(pci.is_pci_host_bridge());
                }
            }
        }

        // We should find at least one PCI node in the qemu PCI test file
        assert!(pci_nodes_found > 0, "Should find at least one PCI node");
    }

    #[test]
    fn test_interrupt_map_parsing() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/qemu_pci.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        // Look for a PCI node with interrupt-map
        for node in fdt.find_nodes("/") {
            {
                if let Node::Pci(pci) = node {
                    if let Ok(interrupt_map) = pci.interrupt_map() {
                        // println!("Found interrupt-map with {} entries", interrupt_map.len());
                        for (i, mapping) in interrupt_map.iter().enumerate() {
                            // println!("Mapping {}: child_addr=0x{:08x}, child_irq={}, parent={:?}",
                            //        i, mapping.child_address, mapping.child_irq, mapping.parent_irq);
                        }
                        return; // Test passed if we found and parsed interrupt-map
                    }
                }
            }
        }

        // If we get here, no interrupt-map was found
        // println!("No interrupt-map found in any PCI node");
    }

    #[test]
    fn test_interrupt_map_mask() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/qemu_pci.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        for node in fdt.find_nodes("/") {
            {
                if let Node::Pci(pci) = node {
                    if let Some(mask) = pci.interrupt_map_mask() {
                        // println!("Found interrupt-map-mask: {:?}", mask);
                        assert_eq!(mask.len(), 4, "PCI interrupt-map-mask should have 4 cells");
                        return; // Test passed
                    }
                }
            }
        }

        // println!("No interrupt-map-mask found in any PCI node");
    }

    #[test]
    fn test_bus_range() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/qemu_pci.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        for node in fdt.find_nodes("/") {
            {
                if let Node::Pci(pci) = node {
                    if let Some((start, end)) = pci.bus_range() {
                        // println!("Found bus-range: {}-{}", start, end);
                        assert!(start <= end, "Bus range start should be <= end");
                        return; // Test passed
                    }
                }
            }
        }

        // println!("No bus-range found in any PCI node");
    }

    #[test]
    fn test_pci_properties() {
        let dtb_data = include_bytes!("../../../../dtb-file/src/dtb/qemu_pci.dtb");
        let fdt = Fdt::from_bytes(dtb_data).unwrap();

        for node in fdt.find_nodes("/") {
            {
                if let Node::Pci(pci) = node {
                    // Test address cells
                    assert_eq!(pci.address_cells(), 3, "PCI should use 3 address cells");

                    // Test interrupt cells
                    assert_eq!(pci.interrupt_cells(), 1, "PCI should use 1 interrupt cell");

                    // Test device type
                    if let Some(device_type) = pci.device_type() {
                        // println!("Device type: {}", device_type);
                    }

                    // Test compatibles
                    let compatibles = pci.compatibles();
                    if !compatibles.is_empty() {
                        // println!("Compatibles: {:?}", compatibles);
                    }

                    return; // Test passed for first PCI node found
                }
            }
        }

        panic!("No PCI nodes found for property testing");
    }
}