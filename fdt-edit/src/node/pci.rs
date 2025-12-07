use core::ops::{Deref, Range};

use alloc::vec::Vec;
use fdt_raw::{FdtError, Phandle, data::U32Iter};
use log::debug;

use crate::node::gerneric::NodeRefGen;

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

#[derive(Clone, Debug)]
pub struct NodeRefPci<'a> {
    pub node: NodeRefGen<'a>,
}

impl<'a> NodeRefPci<'a> {
    // 在这里添加 PCI 相关的方法
    pub fn try_from(node: NodeRefGen<'a>) -> Result<Self, NodeRefGen<'a>> {
        if node.device_type() == Some("pci") {
            Ok(Self { node })
        } else {
            Err(node)
        }
    }

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

    /// 获取 PCI 设备的中断信息
    /// 参数: bus, device, function, pin (1=INTA, 2=INTB, 3=INTC, 4=INTD)
    pub fn child_interrupts(
        &self,
        bus: u8,
        device: u8,
        function: u8,
        interrupt_pin: u8,
    ) -> Result<PciInterruptInfo, FdtError> {
        // 获取 interrupt-map 和 mask
        let interrupt_map = self.interrupt_map()?;

        // 将 mask 转换为 Vec 以便索引访问
        let mask: Vec<u32> = self
            .interrupt_map_mask()
            .ok_or(FdtError::NotFound)?
            .collect();

        // 构造 PCI 设备的子地址
        // 格式: [bus_num, device_num, func_num] 在适当的位
        let child_addr_high = ((bus as u32 & 0xff) << 16)
            | ((device as u32 & 0x1f) << 11)
            | ((function as u32 & 0x7) << 8);
        let child_addr_mid = 0u32;
        let child_addr_low = 0u32;

        let child_addr_cells = self.address_cells().unwrap_or(3) as usize;
        let child_irq_cells = self.interrupt_cells() as usize;

        let encoded_address = [child_addr_high, child_addr_mid, child_addr_low];
        let mut masked_child_address = Vec::with_capacity(child_addr_cells);

        // 应用 mask 到子地址
        for (idx, value) in encoded_address.iter().take(child_addr_cells).enumerate() {
            let mask_value = mask.get(idx).copied().unwrap_or(0xffff_ffff);
            masked_child_address.push(value & mask_value);
        }

        // 如果 encoded_address 比 child_addr_cells 短，填充 0
        let remaining = child_addr_cells.saturating_sub(encoded_address.len());
        masked_child_address.extend(core::iter::repeat_n(0, remaining));

        let encoded_irq = [interrupt_pin as u32];
        let mut masked_child_irq = Vec::with_capacity(child_irq_cells);

        // 应用 mask 到子 IRQ
        for (idx, value) in encoded_irq.iter().take(child_irq_cells).enumerate() {
            let mask_value = mask
                .get(child_addr_cells + idx)
                .copied()
                .unwrap_or(0xffff_ffff);
            masked_child_irq.push(value & mask_value);
        }

        // 如果 encoded_irq 比 child_irq_cells 短，填充 0
        let remaining_irq = child_irq_cells.saturating_sub(encoded_irq.len());
        masked_child_irq.extend(core::iter::repeat_n(0, remaining_irq));

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
    pub fn interrupt_map(&self) -> Result<Vec<PciInterruptMap>, FdtError> {
        let prop = self
            .find_property("interrupt-map")
            .ok_or(FdtError::NotFound)?;

        // 将 mask 和 data 转换为 Vec 以便索引访问
        let mask: Vec<u32> = self
            .interrupt_map_mask()
            .ok_or(FdtError::NotFound)?
            .collect();

        let mut data = prop.as_reader();
        let mut mappings = Vec::new();

        // 计算每个条目的大小
        // 格式: <child-address child-irq interrupt-parent parent-address parent-irq...>
        let child_addr_cells = self.address_cells().unwrap_or(3) as usize;
        let child_irq_cells = self.interrupt_cells() as usize;

        loop {
            // 解析子地址
            let mut child_address = Vec::with_capacity(child_addr_cells);
            for _ in 0..child_addr_cells {
                match data.read_u32() {
                    Some(v) => child_address.push(v),
                    None => return Ok(mappings), // 数据结束
                }
            }

            // 解析子 IRQ
            let mut child_irq = Vec::with_capacity(child_irq_cells);
            for _ in 0..child_irq_cells {
                match data.read_u32() {
                    Some(v) => child_irq.push(v),
                    None => return Ok(mappings),
                }
            }

            // 解析中断父 phandle
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

            // 通过 phandle 查找中断父节点以获取其 #address-cells 和 #interrupt-cells
            // 根据 devicetree 规范，interrupt-map 中的 parent unit address 使用中断父节点的 #address-cells
            let (parent_addr_cells, parent_irq_cells) =
                if let Some(irq_parent) = self.ctx.find_by_phandle(interrupt_parent) {
                    debug!("Found interrupt parent: {:?}", irq_parent.name);

                    // 直接使用中断父节点的 #address-cells
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
                    // 默认值：address_cells=0, interrupt_cells=3 (GIC 格式)
                    (0, 3)
                };

            // 跳过父地址 cells
            for _ in 0..parent_addr_cells {
                if data.read_u32().is_none() {
                    return Ok(mappings);
                }
            }

            // 解析父 IRQ
            let mut parent_irq = Vec::with_capacity(parent_irq_cells);
            for _ in 0..parent_irq_cells {
                match data.read_u32() {
                    Some(v) => parent_irq.push(v),
                    None => return Ok(mappings),
                }
            }

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
    }
}

impl<'a> Deref for NodeRefPci<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
