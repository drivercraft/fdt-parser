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

            for addr in child_addr.iter_mut() {
                *addr = data.pop_front()?;
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

    /// 获取 PCI 设备的中断信息
    /// 参数: bus, device, function, pin (1=INTA, 2=INTB, 3=INTC, 4=INTD)
    pub fn child_interrupts(
        &self,
        ctx: &FdtContext,
        bus: u8,
        device: u8,
        function: u8,
        interrupt_pin: u8,
    ) -> Result<PciInterruptInfo, FdtError> {
        // 获取 interrupt-map 和 mask
        let interrupt_map = self.interrupt_map(ctx)?;

        let mut mask = self.interrupt_map_mask().ok_or(FdtError::NotFound)?;

        // 构造 PCI 设备的子地址
        // 格式: [bus_num, device_num, func_num] 在适当的位
        let child_addr_high = ((bus as u32 & 0xff) << 16)
            | ((device as u32 & 0x1f) << 11)
            | ((function as u32 & 0x7) << 8);
        let child_addr_mid = 0u32;
        let child_addr_low = 0u32;

        let child_addr_cells = self.address_cells().unwrap_or(3) as usize;
        let child_irq_cells = self.interrupt_cells() as usize;
        let required_mask_len = child_addr_cells + child_irq_cells;
        if mask.len() < required_mask_len {
            mask.resize(required_mask_len, 0xffff_ffff);
        }

        let encoded_address = [child_addr_high, child_addr_mid, child_addr_low];
        let mut masked_child_address = Vec::with_capacity(child_addr_cells);

        // 使用迭代器替代不必要的范围循环
        for (idx, value) in encoded_address.iter().enumerate() {
            masked_child_address.push(value & mask[idx]);
        }

        // 如果 encoded_address 比 mask 短，处理剩余的 mask 值
        if encoded_address.len() < child_addr_cells {
            // 如果 encoded_address 比 mask 短，填充剩余的 0 值
            let remaining_zeros = child_addr_cells - encoded_address.len();
            masked_child_address.extend(core::iter::repeat_n(0, remaining_zeros));
        }

        let encoded_irq = [interrupt_pin as u32];
        let mut masked_child_irq = Vec::with_capacity(child_irq_cells);

        // 使用迭代器替代不必要的范围循环
        let mask_start = child_addr_cells;
        let mask_end = child_addr_cells + encoded_irq.len().min(child_irq_cells);
        for (value, mask_value) in encoded_irq.iter().zip(&mask[mask_start..mask_end]) {
            masked_child_irq.push(value & mask_value);
        }

        // 如果 encoded_irq 比 child_irq_cells 短，处理剩余的 mask 值
        if encoded_irq.len() < child_irq_cells {
            let remaining_zeros = child_irq_cells - encoded_irq.len();
            masked_child_irq.extend(core::iter::repeat_n(0, remaining_zeros));
        }

        // 在 interrupt-map 中查找匹配的条目
        for mapping in &interrupt_map {
            if mapping.child_address == masked_child_address
                && mapping.child_irq == masked_child_irq
            {
                return Ok(PciInterruptInfo {
                    irqs: mapping.parent_irq.clone(),
                });
            }
        }

        // 回退到简单的 IRQ 计算
        let simple_irq = (device as u32 * 4 + interrupt_pin as u32) % 32;
        Ok(PciInterruptInfo {
            irqs: vec![simple_irq],
        })
    }

    /// 解析 interrupt-map 属性
    pub fn interrupt_map(&self, ctx: &FdtContext) -> Result<Vec<PciInterruptMap>, FdtError> {
        let prop = self
            .find_property("interrupt-map")
            .ok_or(FdtError::NotFound)?;

        let PropertyKind::Raw(raw) = &prop.kind else {
            return Err(FdtError::NotFound);
        };

        let mut mask = self.interrupt_map_mask().ok_or(FdtError::NotFound)?;

        let data = raw.as_u32_vec();
        let mut mappings = Vec::new();

        // 计算每个条目的大小
        // 格式: <child-address child-irq interrupt-parent parent-address parent-irq...>
        let child_addr_cells = self.address_cells().unwrap_or(3) as usize;
        let child_irq_cells = self.interrupt_cells() as usize;

        let required_mask_len = child_addr_cells + child_irq_cells;
        if mask.len() < required_mask_len {
            mask.resize(required_mask_len, 0xffff_ffff);
        }

        let mut idx = 0;
        while idx < data.len() {
            // 解析子地址
            if idx + child_addr_cells > data.len() {
                break;
            }
            let child_address = data[idx..idx + child_addr_cells].to_vec();
            idx += child_addr_cells;

            // 解析子 IRQ
            if idx + child_irq_cells > data.len() {
                break;
            }
            let child_irq = data[idx..idx + child_irq_cells].to_vec();
            idx += child_irq_cells;

            // 解析中断父 phandle
            if idx >= data.len() {
                break;
            }
            let interrupt_parent_raw = data[idx];
            let interrupt_parent = Phandle::from(interrupt_parent_raw);
            idx += 1;

            // 通过 phandle 查找中断父节点以获取其 address_cells 和 interrupt_cells
            let (parent_addr_cells, parent_irq_cells) =
                if let Some(irq_parent) = ctx.find_by_phandle(interrupt_parent) {
                    let addr_cells = irq_parent.address_cells().unwrap_or(0) as usize;
                    let irq_cells = irq_parent
                        .find_property("#interrupt-cells")
                        .and_then(|p| match &p.kind {
                            PropertyKind::Num(v) => Some(*v as usize),
                            _ => None,
                        })
                        .unwrap_or(3);
                    (addr_cells, irq_cells)
                } else {
                    // 默认值：address_cells=0, interrupt_cells=3 (GIC 格式)
                    (0, 3)
                };

            // 跳过父地址 cells
            if idx + parent_addr_cells > data.len() {
                break;
            }
            idx += parent_addr_cells;

            // 解析父 IRQ
            if idx + parent_irq_cells > data.len() {
                break;
            }
            let parent_irq = data[idx..idx + parent_irq_cells].to_vec();
            idx += parent_irq_cells;

            // 应用 mask 到子地址和 IRQ
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

        Ok(mappings)
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
