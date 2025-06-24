use core::iter;

use crate::{
    clocks::{ClockRef, ClocksIter},
    error::{FdtError, FdtResult},
    interrupt::InterruptController,
    meta::MetaData,
    pci::Pci,
    property::Property,
    read::{FdtReader, U32Array2D},
    Fdt, FdtRangeSilce, FdtReg, Phandle, Status, Token,
};

#[derive(Clone)]
pub struct Node<'a> {
    pub level: usize,
    pub name: &'a str,
    pub(crate) fdt: Fdt<'a>,
    /// 父节点的元数据
    pub(crate) meta_parents: MetaData<'a>,
    /// 当前节点的元数据
    pub(crate) meta: MetaData<'a>,
    body: FdtReader<'a>,
}

impl<'a> Node<'a> {
    pub(crate) fn new(
        fdt: &Fdt<'a>,
        level: usize,
        name: &'a str,
        reader: FdtReader<'a>,
        meta_parents: MetaData<'a>,
        meta: MetaData<'a>,
    ) -> Self {
        Self {
            fdt: fdt.clone(),
            level,
            body: reader,
            name,
            meta,
            meta_parents,
        }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn propertys(&self) -> impl Iterator<Item = Property<'a>> + '_ {
        let reader = self.body.clone();
        PropIter {
            reader,
            fdt: self.fdt.clone(),
        }
    }

    pub fn find_property(&self, name: &str) -> Option<Property<'a>> {
        self.propertys().find(|x| x.name.eq(name))
    }

    pub fn reg(&self) -> Option<impl Iterator<Item = FdtReg> + 'a> {
        let mut iter = self.propertys();
        let reg = iter.find(|x| x.name.eq("reg"))?;

        Some(RegIter {
            size_cell: self.meta_parents.size_cells.unwrap(),
            address_cell: self.meta_parents.address_cells.unwrap(),
            prop: reg,
            ranges: self.meta_parents.range.clone(),
        })
    }

    pub(crate) fn node_ranges(&self) -> Option<FdtRangeSilce<'a>> {
        let prop = self.find_property("ranges")?;

        Some(FdtRangeSilce::new(
            self.meta.address_cells.unwrap(),
            self.meta_parents.address_cells.unwrap(),
            self.meta.size_cells.unwrap(),
            prop.data.clone(),
        ))
    }

    pub(crate) fn node_interrupt_parent(&self) -> Option<Phandle> {
        let prop = self.find_property("interrupt-parent")?;
        Some(prop.u32().into())
    }

    /// Find [InterruptController] from current node or its parent
    pub fn interrupt_parent(&self) -> Option<InterruptController<'a>> {
        let phandle = if let Some(p) = self.meta.interrupt_parent {
            Some(p)
        } else {
            self.meta_parents.interrupt_parent
        }?;

        self.fdt
            .get_node_by_phandle(phandle)
            .map(|node| InterruptController { node })
    }

    pub fn compatible(&self) -> Option<impl Iterator<Item = FdtResult<'a, &'a str>> + 'a> {
        let prop = self.find_property("compatible")?;
        let mut value = prop.data.clone();

        Some(iter::from_fn(move || {
            let s = value.take_str();
            match s {
                Ok(s) => {
                    if s.is_empty() {
                        None
                    } else {
                        Some(Ok(s))
                    }
                }
                Err(e) => match e {
                    FdtError::Eof => None,
                    _ => Some(Err(e)),
                },
            }
        }))
    }

    /// Get all compatible ignoring errors
    pub fn compatibles(&self) -> impl Iterator<Item = &'a str> + 'a {
        let mut cap_raw = self.compatible();

        iter::from_fn(move || {
            if let Some(caps) = &mut cap_raw {
                let cap = caps.next()?.ok()?;
                Some(cap)
            } else {
                None
            }
        })
    }

    pub fn phandle(&self) -> Option<Phandle> {
        let prop = self.find_property("phandle")?;
        Some(prop.u32().into())
    }

    pub fn interrupts(&self) -> Option<impl Iterator<Item = impl Iterator<Item = u32> + 'a> + 'a> {
        let prop = self.find_property("interrupts")?;
        let cell_size = self.interrupt_parent()?.interrupt_cells();

        Some(U32Array2D::new(prop.raw_value(), cell_size))
    }

    pub fn clocks(&'a self) -> impl Iterator<Item = ClockRef<'a>> + 'a {
        ClocksIter::new(self)
    }

    pub fn clock_frequency(&self) -> Option<u32> {
        let prop = self.find_property("clock-frequency")?;
        Some(prop.u32())
    }

    pub fn into_pci(self) -> Option<Pci<'a>> {
        if self.name.contains("pci") {
            Some(Pci { node: self })
        } else {
            None
        }
    }

    pub fn status(&self) -> Option<Status> {
        let prop = self.find_property("status")?;
        let s = prop.str();

        if s.contains("disabled") {
            return Some(Status::Disabled);
        }

        if s.contains("okay") {
            return Some(Status::Okay);
        }

        None
    }

    pub fn fdt(&self) -> Fdt<'a> {
        self.fdt.clone()
    }
}

struct RegIter<'a> {
    size_cell: u8,
    address_cell: u8,
    prop: Property<'a>,
    ranges: Option<FdtRangeSilce<'a>>,
}
impl Iterator for RegIter<'_> {
    type Item = FdtReg;

    fn next(&mut self) -> Option<Self::Item> {
        let child_bus_address = self.prop.data.take_by_cell_size(self.address_cell)?;

        let mut address = child_bus_address;

        if let Some(ranges) = &self.ranges {
            for one in ranges.iter() {
                let range_child_bus_address = one.child_bus_address().as_u64();
                let range_parent_bus_address = one.parent_bus_address().as_u64();

                if child_bus_address >= range_child_bus_address
                    && child_bus_address < range_child_bus_address + one.size
                {
                    address =
                        child_bus_address - range_child_bus_address + range_parent_bus_address;
                    break;
                }
            }
        }

        let size = if self.size_cell > 0 {
            Some(self.prop.data.take_by_cell_size(self.size_cell)? as usize)
        } else {
            None
        };
        Some(FdtReg {
            address,
            child_bus_address,
            size,
        })
    }
}

struct PropIter<'a> {
    fdt: Fdt<'a>,
    reader: FdtReader<'a>,
}

impl<'a> Iterator for PropIter<'a> {
    type Item = Property<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.reader.take_token() {
                Some(token) => match token {
                    Token::Prop => break,
                    Token::Nop => {}
                    _ => return None,
                },
                None => return None,
            }
        }
        self.reader.take_prop(&self.fdt)
    }
}

// #[derive(Clone)]
// pub struct MemoryRegionSilce<'a> {
//     address_cell: u8,
//     size_cell: u8,
//     reader: FdtReader<'a>,
// }

// impl<'a> MemoryRegionSilce<'a> {
//     pub fn iter(&self) -> impl Iterator<Item = FdtRange> + 'a {
//         MemoryRegionIter {
//             address_cell: self.address_cell,
//             size_cell: self.size_cell,
//             reader: self.reader.clone(),
//         }
//     }
// }

// struct MemoryRegionIter<'a> {
//     address_cell: u8,
//     size_cell: u8,
//     reader: FdtReader<'a>,
// }

// impl<'a> Iterator for MemoryRegionIter<'a> {
//     type Item = FdtRange;

//     fn next(&mut self) -> Option<Self::Item> {
//         todo!()
//     }
// }
