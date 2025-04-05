use core::{fmt::Debug, ops::Range};

use crate::{
    define::Phandle,
    error::FdtResult,
    node::Node,
    read::{FdtReader, U32Array},
    FdtError, FdtRangeIter, InterruptController,
};

pub struct Pci<'a> {
    pub node: Node<'a>,
}

impl<'a> Pci<'a> {
    pub fn bus_range(&self) -> Option<Range<usize>> {
        let prop = self.node.find_property("bus-range")?;
        let mut reader = FdtReader::new(prop.raw_value());
        let start = reader.take_u32()?;
        let end = reader.take_u32()?;

        Some(start as usize..end as usize)
    }

    pub fn ranges(&self) -> FdtResult<impl Iterator<Item = PciRange> + 'a> {
        let ranges = self
            .node
            .node_ranges()
            .ok_or(FdtError::NotFound("ranges"))?;

        let iter = ranges.iter();

        Ok(PciRangeIter { iter })
    }

    pub fn child_interrupts(
        &self,
        bus: u8,
        device: u8,
        func: u8,
        irq_pin: u32,
    ) -> FdtResult<PciChildIrq<'a>> {
        let mask = self.interrupt_map_mask()?;

        let want0 = ((bus as u32) << 16) | ((device as u32) << 11) | ((func as u32) << 8);

        let mut want = [0; 4];

        want[0] = want0 & mask[0];
        want[3] = irq_pin & mask[3];

        let mut prop = self
            .node
            .find_property("interrupt-map")
            .ok_or(FdtError::NotFound("interrupt-map"))?;

        while let Some(hi) = prop.data.take_u32() {
            let _mid = prop.data.take_u32().ok_or(FdtError::Eof)?;
            let _lo = prop.data.take_u32().ok_or(FdtError::Eof)?;
            let irq_line = prop.data.take_u32().ok_or(FdtError::Eof)?;

            let parent = Phandle::from(prop.data.take_u32().ok_or(FdtError::Eof)?);

            let parent_node = self
                .node
                .fdt
                .get_node_by_phandle(parent)
                .ok_or(FdtError::NotFound("parent interrupt"))?;

            let address_cell = parent_node
                .find_property("#address-cells")
                .map(|p| p.u32())
                .unwrap_or_default();

            for _i in 0..address_cell {
                prop.data.take_u32().ok_or(FdtError::Eof)?;
            }

            let parent_node = InterruptController { node: parent_node };

            let cell_size = parent_node.interrupt_cells();

            let data = prop
                .data
                .take(cell_size * size_of::<u32>())
                .ok_or(FdtError::Eof)?;

            if hi & mask[0] != want[0] || irq_line != want[3] {
                continue;
            }

            return Ok(PciChildIrq {
                parent,
                irqs: U32Array::new(data),
            });
        }

        Err(FdtError::NotFound("pci child"))
    }

    fn interrupt_map_mask(&self) -> FdtResult<[u32; 4]> {
        let prop = self
            .node
            .find_property("interrupt-map-mask")
            .ok_or(FdtError::NotFound("interrupt-map-mask"))?;

        let mut mask = [0u32; 4];
        let mut data = prop.data.clone();

        for one in mask.iter_mut() {
            *one = data.take_u32().ok_or(FdtError::Eof)?;
        }

        Ok(mask)
    }
}

pub struct PciChildIrq<'a> {
    pub parent: Phandle,
    pub irqs: U32Array<'a>,
}

pub struct PciRangeIter<'a> {
    iter: FdtRangeIter<'a>,
}

impl Iterator for PciRangeIter<'_> {
    type Item = PciRange;

    fn next(&mut self) -> Option<Self::Item> {
        let one = self.iter.next()?;
        let mut child = one.child_bus_address();
        let cpu_address = one.parent_bus_address().as_u64();
        let size = one.size;

        let hi = child.next().unwrap();
        let mid = child.next().unwrap();
        let low = child.next().unwrap();

        let ss = (hi >> 24) & 0b11;
        let prefetchable = (hi & (1 << 30)) > 0;

        let space = match ss {
            0b00 => PciSpace::Configuration,
            0b01 => PciSpace::IO,
            0b10 => PciSpace::Memory32,
            0b11 => PciSpace::Memory64,
            _ => panic!(),
        };

        let child_bus_address = ((mid as u64) << 32) | low as u64;

        Some(PciRange {
            space,
            bus_address: child_bus_address,
            cpu_address,
            size,
            prefetchable,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciSpace {
    Configuration,
    IO,
    Memory32,
    Memory64,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PciRange {
    pub space: PciSpace,
    pub bus_address: u64,
    pub cpu_address: u64,
    pub size: u64,
    pub prefetchable: bool,
}

// #[derive(Debug, Clone, Copy)]
// pub struct PciPhys {
//     pub bus_address: u64,
//     pub bus_num: u8,
//     pub device_num: u8,
//     pub function_num: u8,
//     pub register_num: u8,
//     pub space: PciSpace,
//     pub prefetchable: bool,
//     pub relocatable: bool,
// }

// impl From<[u32; 3]> for PciPhys {
//     fn from(value: [u32; 3]) -> Self {
//         // [hi, mid, low]

//         let hi = value[0];
//         let mid = value[1];
//         let low = value[2];

//         let ss = (hi >> 24) & 0b11;
//         let prefetchable = (hi & 1 << 30) > 0;
//         let relocatable = (hi & 1 << 31) > 0;

//         let bus_num = ((hi >> 16) & 0xFF) as u8;

//         let device_num = ((hi >> 11) & 0b11111) as u8;
//         let function_num = (hi >> 8 & 0b111) as u8;
//         let register_num = (hi & 0xFF) as u8;

//         let space = match ss {
//             0b00 => PciSpace::Configuration,
//             0b01 => PciSpace::IO,
//             0b10 => PciSpace::Memory32,
//             0b11 => PciSpace::Memory64,
//             _ => panic!(),
//         };

//         let child_bus_address = (mid as u64) << 32 | low as u64;

//         PciPhys {
//             space,
//             bus_address: child_bus_address,
//             prefetchable,
//             relocatable,
//             bus_num,
//             device_num,
//             function_num,
//             register_num,
//         }
//     }
// }

impl Debug for PciRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PciRange {{ space: {:?}, child_bus_address: {:#x}, parent_bus_address: {:#x}, size: {:#x}, prefetchable: {}}}", 
        self.space, self.bus_address, self.cpu_address, self.size, self.prefetchable)
    }
}
