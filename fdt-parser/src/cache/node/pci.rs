use core::{
    fmt::Debug,
    ops::{Deref, Range},
};

use crate::{cache::node::NodeBase, FdtError, Phandle};
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
    pub child_address: Vec<u32>,
    pub child_irq: Vec<u32>,
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

    pub fn interrupt_cells(&self) -> u32 {
        self.find_property("#interrupt-cells")
            .and_then(|prop| prop.u32().ok())
            .unwrap_or(1) // Default to 1 interrupt cell for PCI
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
        let prop = self
            .node
            .find_property("interrupt-map")
            .ok_or(FdtError::PropertyNotFound("interrupt-map"))?;

        let mut mask = self
            .interrupt_map_mask()
            .ok_or(FdtError::PropertyNotFound("interrupt-map-mask"))?;

        let mut data = prop.data.buffer();
        let mut mappings = Vec::new();

        // Calculate the size of each entry in interrupt-map
        // Format: <child-address child-irq interrupt-parent parent-irq...>
        let child_addr_cells = self.address_cells() as usize;
        let child_irq_cells = self.interrupt_cells() as usize;

        let required_mask_len = child_addr_cells + child_irq_cells;
        if mask.len() < required_mask_len {
            mask.resize(required_mask_len, 0xffff_ffff);
        }

        while !data.remain().as_ref().is_empty() {
            // Parse child address (variable number of cells for PCI)
            let mut child_address = Vec::with_capacity(child_addr_cells);
            for _ in 0..child_addr_cells {
                child_address.push(
                    data.take_u32()
                        .map_err(|_| FdtError::BufferTooSmall { pos: data.pos() })?,
                );
            }

            // Parse child IRQ (usually 1 cell for PCI)
            let mut child_irq = Vec::with_capacity(child_irq_cells);
            for _ in 0..child_irq_cells {
                child_irq.push(
                    data.take_u32()
                        .map_err(|_| FdtError::BufferTooSmall { pos: data.pos() })?,
                );
            }

            // Parse interrupt parent phandle
            let interrupt_parent_raw = data
                .take_u32()
                .map_err(|_| FdtError::BufferTooSmall { pos: data.pos() })?;
            let interrupt_parent = if interrupt_parent_raw == 0 {
                self.interrupt_parent_phandle().unwrap_or(Phandle::from(0))
            } else {
                Phandle::from(interrupt_parent_raw)
            };

            let irq_parent = self
                .node
                .interrupt_parent()
                .ok_or(FdtError::NodeNotFound("interrupt-parent"))?;

            let address_cells = irq_parent.address_cells();

            for _ in 0..address_cells {
                data.take_u32()
                    .map_err(|_| FdtError::BufferTooSmall { pos: data.pos() })?;
            }

            let parent_irq_cells = irq_parent.interrupt_cells()? as usize;

            // Parse parent IRQ (variable number of cells)
            let mut parent_irq = Vec::with_capacity(parent_irq_cells);
            for _ in 0..parent_irq_cells {
                let irq = data
                    .take_u32()
                    .map_err(|_| FdtError::BufferTooSmall { pos: data.pos() })?;
                parent_irq.push(irq);
            }

            // Apply mask to child address and IRQ
            let masked_address: Vec<u32> = child_address
                .iter()
                .enumerate()
                .map(|(idx, value)| {
                    let mask_value = mask.get(idx).copied().unwrap_or(0xffff_ffff);
                    value & mask_value
                })
                .collect();
            let masked_irq: Vec<u32> = child_irq
                .iter()
                .enumerate()
                .map(|(idx, value)| {
                    let mask_value = mask
                        .get(child_addr_cells + idx)
                        .copied()
                        .unwrap_or(0xffff_ffff);
                    value & mask_value
                })
                .collect();

            mappings.push(PciInterruptMap {
                child_address: masked_address,
                child_irq: masked_irq,
                interrupt_parent,
                parent_irq,
            });
        }

        Ok(mappings)
    }

    /// Get the bus range property if present
    pub fn bus_range(&self) -> Option<Range<u32>> {
        self.node.find_property("bus-range").and_then(|prop| {
            let mut data = prop.data.buffer();
            let start = data.take_u32().ok()?;
            let end = data.take_u32().unwrap_or(start);
            Some(start..end)
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
        self.device_type() == Some("pci")
            || self.node.name().contains("pci")
            || self.node.compatibles().iter().any(|c| c.contains("pci"))
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

    /// Get interrupt information for a specific PCI device
    /// Parameters: bus, device, function, pin (0=INTA, 1=INTB, 2=INTC, 3=INTD)
    pub fn child_interrupts(
        &self,
        bus: u32,
        device: u32,
        function: u32,
        pin: u32,
    ) -> Result<PciInterruptInfo, FdtError> {
        // Try to get interrupt-map and mask, fall back to simpler approach if parsing fails
        let interrupt_map = self.interrupt_map()?;

        let mut mask = self
            .interrupt_map_mask()
            .ok_or(FdtError::PropertyNotFound("interrupt-map-mask"))?;

        // Construct the child address for PCI device
        // Format: [bus_num, device_num, func_num] in appropriate bits
        let child_addr_high =
            ((bus & 0xff) << 16) | ((device & 0x1f) << 11) | ((function & 0x7) << 8);
        let child_addr_mid = 0;
        let child_addr_low = 0;

        let child_addr_cells = self.address_cells() as usize;
        let child_irq_cells = self.interrupt_cells() as usize;
        let required_mask_len = child_addr_cells + child_irq_cells;
        if mask.len() < required_mask_len {
            mask.resize(required_mask_len, 0xffff_ffff);
        }

        let encoded_address = [child_addr_high, child_addr_mid, child_addr_low];
        let mut masked_child_address = Vec::with_capacity(child_addr_cells);
        for idx in 0..child_addr_cells {
            let value = *encoded_address.get(idx).unwrap_or(&0);
            masked_child_address.push(value & mask[idx]);
        }

        let encoded_irq = [pin];
        let mut masked_child_irq = Vec::with_capacity(child_irq_cells);
        for idx in 0..child_irq_cells {
            let value = *encoded_irq.get(idx).unwrap_or(&0);
            masked_child_irq.push(value & mask[child_addr_cells + idx]);
        }

        // Look for matching entry in interrupt-map
        for mapping in &interrupt_map {
            // Check if this mapping matches our masked address and pin
            if mapping.child_address == masked_child_address
                && mapping.child_irq == masked_child_irq
            {
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
    use crate::{cache::node::Node, Fdt};

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
                        assert!(!interrupt_map.is_empty());
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
                    if let Some(range) = pci.bus_range() {
                        // println!("Found bus-range: {}-{}", start, end);
                        assert!(range.start <= range.end, "Bus range start should be <= end");
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
                        assert!(!device_type.is_empty());
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
