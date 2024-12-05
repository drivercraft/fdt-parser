use core::{fmt::Debug, ops::Range};

use crate::{error::FdtResult, node::Node, read::FdtReader, FdtError, FdtRangeIter};

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

    pub fn ranges(&'a self) -> FdtResult<impl Iterator<Item = PciRange> + 'a> {
        let ranges = self
            .node
            .node_ranges()
            .ok_or(FdtError::NotFound("ranges"))?;

        let iter = ranges.iter();

        Ok(PciRangeIter { iter })
    }
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
        let prefetchable = (hi & 1 << 30) > 0;

        let space = match ss {
            0b00 => PciSpace::Configuration,
            0b01 => PciSpace::IO,
            0b10 => PciSpace::Memory32,
            0b11 => PciSpace::Memory64,
            _ => panic!(),
        };

        let child_bus_address = (mid as u64) << 32 | low as u64;

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

impl Debug for PciRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PciRange {{ space: {:?}, child_bus_address: {:#x}, parent_bus_address: {:#x}, size: {:#x}, prefetchable: {}}}", 
        self.space, self.bus_address, self.cpu_address, self.size, self.prefetchable)
    }
}
