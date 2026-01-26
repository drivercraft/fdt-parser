use core::ops::{Deref, Range};

use alloc::vec::Vec;
use fdt_raw::{FdtError, Phandle, data::U32Iter};
use log::debug;

use crate::node::gerneric::NodeRefGen;

/// PCI address space types.
#[derive(Clone, Debug, PartialEq)]
pub enum PciSpace {
    /// I/O space
    IO,
    /// 32-bit memory space
    Memory32,
    /// 64-bit memory space
    Memory64,
}

/// PCI address range entry.
///
/// Represents a range of addresses in PCI address space with mapping to CPU address space.
#[derive(Clone, Debug, PartialEq)]
pub struct PciRange {
    /// The PCI address space type
    pub space: PciSpace,
    /// Address on the PCI bus
    pub bus_address: u64,
    /// Address in CPU physical address space
    pub cpu_address: u64,
    /// Size of the range in bytes
    pub size: u64,
    /// Whether the memory region is prefetchable
    pub prefetchable: bool,
}

/// PCI interrupt mapping entry.
///
/// Represents a mapping from PCI device interrupts to parent interrupt controller inputs.
#[derive(Clone, Debug)]
pub struct PciInterruptMap {
    /// Child device address (masked)
    pub child_address: Vec<u32>,
    /// Child device IRQ (masked)
    pub child_irq: Vec<u32>,
    /// Phandle of the interrupt parent controller
    pub interrupt_parent: Phandle,
    /// Parent controller IRQ inputs
    pub parent_irq: Vec<u32>,
}

/// PCI interrupt information.
///
/// Contains the resolved interrupt information for a PCI device.
#[derive(Clone, Debug, PartialEq)]
pub struct PciInterruptInfo {
    /// List of IRQ numbers
    pub irqs: Vec<u32>,
}

/// PCI node reference.
///
/// Provides specialized access to PCI bridge nodes and their properties.
#[derive(Clone, Debug)]
pub struct NodeRefPci<'a> {
    /// The underlying generic node reference
    pub node: NodeRefGen<'a>,
}

impl<'a> NodeRefPci<'a> {
    /// Attempts to create a PCI node reference from a generic node.
    ///
    /// Returns `Err` with the original node if it's not a PCI node.
    pub fn try_from(node: NodeRefGen<'a>) -> Result<Self, NodeRefGen<'a>> {
        if node.device_type() == Some("pci") {
            Ok(Self { node })
        } else {
            Err(node)
        }
    }

    /// Returns the `#interrupt-cells` property value.
    ///
    /// Defaults to 1 for PCI devices if not specified.
    pub fn interrupt_cells(&self) -> u32 {
        self.find_property("#interrupt-cells")
            .and_then(|prop| prop.get_u32())
            .unwrap_or(1) // Default to 1 interrupt cell for PCI
    }

    /// Get the interrupt-map-mask property if present
    pub fn interrupt_map_mask(&self) -> Option<U32Iter<'_>> {
        self.find_property("interrupt-map-mask")
            .map(|prop| prop.get_u32_iter())
    }

    /// Get the bus range property if present
    pub fn bus_range(&self) -> Option<Range<u32>> {
        self.find_property("bus-range").and_then(|prop| {
            let mut data = prop.get_u32_iter();
            let start = data.next()?;
            let end = data.next()?;

            Some(start..end)
        })
    }

    /// Get the ranges property for address translation
    pub fn ranges(&self) -> Option<Vec<PciRange>> {
        let prop = self.find_property("ranges")?;

        let mut data = prop.as_reader();

        let mut ranges = Vec::new();

        // PCI ranges format: <child-bus-address parent-bus-address size>
        // child-bus-address: 3 cells (pci.hi pci.mid pci.lo) - PCI 地址固定 3 cells
        // parent-bus-address: 使用父节点的 #address-cells
        // size: 使用当前节点的 #size-cells
        let parent_addr_cells = self.ctx.parent_address_cells() as usize;
        let size_cells = self.size_cells().unwrap_or(2) as usize;

        while let Some(pci_hi) = data.read_u32() {
            // Parse child bus address (3 cells for PCI: phys.hi, phys.mid, phys.lo)
            let bus_address = data.read_u64()?;

            // Parse parent bus address (使用父节点的 #address-cells)
            let parent_addr = data.read_cells(parent_addr_cells)?;

            // Parse size (使用当前节点的 #size-cells)
            let size = data.read_cells(size_cells)?;

            // Extract PCI address space and prefetchable from child_addr[0]
            let (space, prefetchable) = self.decode_pci_address_space(pci_hi);

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

    /// Get interrupt information for a PCI device
    /// Parameters: bus, device, function, pin (1=INTA, 2=INTB, 3=INTC, 4=INTD)
    pub fn child_interrupts(
        &self,
        bus: u8,
        device: u8,
        function: u8,
        interrupt_pin: u8,
    ) -> Result<PciInterruptInfo, FdtError> {
        // Get interrupt-map and mask
        let interrupt_map = self.interrupt_map()?;

        // Convert mask to Vec for indexed access
        let mask: Vec<u32> = self
            .interrupt_map_mask()
            .ok_or(FdtError::NotFound)?
            .collect();

        // Construct child address for PCI device
        // Format: [bus_num, device_num, func_num] at appropriate bit positions
        let child_addr_high = ((bus as u32 & 0xff) << 16)
            | ((device as u32 & 0x1f) << 11)
            | ((function as u32 & 0x7) << 8);
        let child_addr_mid = 0u32;
        let child_addr_low = 0u32;

        let child_addr_cells = self.address_cells().unwrap_or(3) as usize;
        let child_irq_cells = self.interrupt_cells() as usize;

        let encoded_address = [child_addr_high, child_addr_mid, child_addr_low];
        let mut masked_child_address = Vec::with_capacity(child_addr_cells);

        // Apply mask to child address
        for (idx, value) in encoded_address.iter().take(child_addr_cells).enumerate() {
            let mask_value = mask.get(idx).copied().unwrap_or(0xffff_ffff);
            masked_child_address.push(value & mask_value);
        }

        // If encoded_address is shorter than child_addr_cells, pad with 0
        let remaining = child_addr_cells.saturating_sub(encoded_address.len());
        masked_child_address.extend(core::iter::repeat_n(0, remaining));

        let encoded_irq = [interrupt_pin as u32];
        let mut masked_child_irq = Vec::with_capacity(child_irq_cells);

        // Apply mask to child IRQ
        for (idx, value) in encoded_irq.iter().take(child_irq_cells).enumerate() {
            let mask_value = mask
                .get(child_addr_cells + idx)
                .copied()
                .unwrap_or(0xffff_ffff);
            masked_child_irq.push(value & mask_value);
        }

        // If encoded_irq is shorter than child_irq_cells, pad with 0
        let remaining_irq = child_irq_cells.saturating_sub(encoded_irq.len());
        masked_child_irq.extend(core::iter::repeat_n(0, remaining_irq));

        // Search for matching entry in interrupt-map
        for mapping in &interrupt_map {
            if mapping.child_address == masked_child_address
                && mapping.child_irq == masked_child_irq
            {
                return Ok(PciInterruptInfo {
                    irqs: mapping.parent_irq.clone(),
                });
            }
        }

        // Fall back to simple IRQ calculation
        let simple_irq = (device as u32 * 4 + interrupt_pin as u32) % 32;
        Ok(PciInterruptInfo {
            irqs: vec![simple_irq],
        })
    }

    /// Parse interrupt-map property
    pub fn interrupt_map(&self) -> Result<Vec<PciInterruptMap>, FdtError> {
        let prop = self
            .find_property("interrupt-map")
            .ok_or(FdtError::NotFound)?;

        // Convert mask and data to Vec for indexed access
        let mask: Vec<u32> = self
            .interrupt_map_mask()
            .ok_or(FdtError::NotFound)?
            .collect();

        let mut data = prop.as_reader();
        let mut mappings = Vec::new();

        // Calculate size of each entry
        // Format: <child-address child-irq interrupt-parent parent-address parent-irq...>
        let child_addr_cells = self.address_cells().unwrap_or(3) as usize;
        let child_irq_cells = self.interrupt_cells() as usize;

        loop {
            // Parse child address
            let mut child_address = Vec::with_capacity(child_addr_cells);
            for _ in 0..child_addr_cells {
                match data.read_u32() {
                    Some(v) => child_address.push(v),
                    None => return Ok(mappings), // End of data
                }
            }

            // Parse child IRQ
            let mut child_irq = Vec::with_capacity(child_irq_cells);
            for _ in 0..child_irq_cells {
                match data.read_u32() {
                    Some(v) => child_irq.push(v),
                    None => return Ok(mappings),
                }
            }

            // Parse interrupt parent phandle
            let interrupt_parent_raw = match data.read_u32() {
                Some(v) => v,
                None => return Ok(mappings),
            };
            let interrupt_parent = Phandle::from(interrupt_parent_raw);

            debug!(
                "Looking for interrupt parent phandle: 0x{:x} (raw: {})",
                interrupt_parent.raw(),
                interrupt_parent_raw
            );
            debug!(
                "Context phandle_map keys: {:?}",
                self.ctx
                    .phandle_map
                    .keys()
                    .map(|p| format!("0x{:x}", p.raw()))
                    .collect::<Vec<_>>()
            );

            // Look up interrupt parent node by phandle to get its #address-cells and #interrupt-cells
            // According to devicetree spec, parent unit address in interrupt-map uses interrupt parent's #address-cells
            let (parent_addr_cells, parent_irq_cells) =
                if let Some(irq_parent) = self.ctx.find_by_phandle(interrupt_parent) {
                    debug!("Found interrupt parent: {:?}", irq_parent.name);

                    // Use interrupt parent node's #address-cells directly
                    let addr_cells = irq_parent.address_cells().unwrap_or(0) as usize;

                    let irq_cells = irq_parent
                        .get_property("#interrupt-cells")
                        .and_then(|p| p.get_u32())
                        .unwrap_or(3) as usize;
                    debug!(
                        "irq_parent addr_cells: {}, irq_cells: {}",
                        addr_cells, irq_cells
                    );
                    (addr_cells, irq_cells)
                } else {
                    debug!(
                        "Interrupt parent phandle 0x{:x} NOT FOUND in context!",
                        interrupt_parent.raw()
                    );
                    // Default values: address_cells=0, interrupt_cells=3 (GIC format)
                    (0, 3)
                };

            // Skip parent address cells
            for _ in 0..parent_addr_cells {
                if data.read_u32().is_none() {
                    return Ok(mappings);
                }
            }

            // Parse parent IRQ
            let mut parent_irq = Vec::with_capacity(parent_irq_cells);
            for _ in 0..parent_irq_cells {
                match data.read_u32() {
                    Some(v) => parent_irq.push(v),
                    None => return Ok(mappings),
                }
            }

            // Apply mask to child address and IRQ
            let masked_address: Vec<u32> = child_address
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    let mask_value = mask.get(i).copied().unwrap_or(0xffff_ffff);
                    value & mask_value
                })
                .collect();
            let masked_irq: Vec<u32> = child_irq
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    let mask_value = mask
                        .get(child_addr_cells + i)
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
    }
}

impl<'a> Deref for NodeRefPci<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
