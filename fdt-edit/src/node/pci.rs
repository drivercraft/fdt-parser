use core::ops::Range;

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use fdt_raw::{FdtError, Phandle};

use crate::{
    FdtContext,
    node::{NodeOp, NodeTrait, RawNode},
    prop::PropertyKind,
};

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
pub struct NodePci(pub(crate) RawNode);

impl NodeOp for NodePci {}

impl NodeTrait for NodePci {
    fn as_raw(&self) -> &RawNode {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawNode {
        &mut self.0
    }

    fn to_raw(self) -> RawNode {
        self.0
    }
}

impl NodePci {
    pub fn new(name: &str) -> Self {
        NodePci(RawNode::new(name))
    }

    pub fn interrupt_cells(&self) -> u32 {
        self.find_property("#interrupt-cells")
            .and_then(|prop| match prop.kind {
                PropertyKind::Num(v) => Some(v as _),
                _ => None,
            })
            .unwrap_or(1) // Default to 1 interrupt cell for PCI
    }

    /// Get the interrupt-map-mask property if present
    pub fn interrupt_map_mask(&self) -> Option<Vec<u32>> {
        self.find_property("interrupt-map-mask").map(|prop| {
            let PropertyKind::Raw(v) = &prop.kind else {
                return Vec::new();
            };
            v.as_u32_vec()
        })
    }

    /// Get the bus range property if present
    pub fn bus_range(&self) -> Option<Range<u32>> {
        self.find_property("bus-range").and_then(|prop| {
            let PropertyKind::Raw(raw) = &prop.kind else {
                return None;
            };
            let data = raw.as_u32_vec();

            if data.len() < 2 {
                return None;
            }
            Some(data[0]..data[1])
        })
    }

    /// Get the ranges property for address translation
    pub fn ranges(&self) -> Option<Vec<PciRange>> {
        let prop = self.find_property("ranges")?;
        let PropertyKind::Raw(raw) = &prop.kind else {
            return None;
        };

        let mut data = VecDeque::from(raw.as_u32_vec());

        let mut ranges = Vec::new();

        // PCI ranges format: <child-bus-address parent-bus-address size>
        // child-bus-address: 3 cells (pci.hi pci.mid pci.lo)
        // parent-bus-address: 2 cells for 64-bit systems (high, low)
        // size: 2 cells for 64-bit sizes (high, low)
        while !data.is_empty() {
            // Parse child bus address (3 cells for PCI)
            let mut child_addr = [0u32; 3];
            for i in 0..3 {
                child_addr[i] = data.pop_front()?;
            }

            // Parse parent bus address (2 cells for 64-bit)
            let parent_addr_high = data.pop_front()?;
            let parent_addr_low = data.pop_front()?;
            let parent_addr = ((parent_addr_high as u64) << 32) | (parent_addr_low as u64);

            // Parse size (2 cells for 64-bit)
            let size_high = data.pop_front()?;
            let size_low = data.pop_front()?;
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

    pub fn child_interrupts(
        &self,
        ctx: &FdtContext,
        bus: u8,
        device: u8,
        function: u8,
        interrupt_pin: u8,
    ) -> Result<PciInterruptInfo, FdtError> {
       
 
    }
}

impl core::fmt::Debug for NodePci {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Pci")
            .field("name", &self.name())
            .field("bus_range", &self.bus_range())
            .field("interrupt_map_mask", &self.interrupt_map_mask())
            .finish()
    }
}
