use core::fmt::Debug;

use crate::{node::Node, read::FdtReader, Fdt, Phandle};

pub struct ClocksIter<'a> {
    pub fdt: Fdt<'a>,
    pub id_list: Option<FdtReader<'a>>,
    pub name_list: Option<FdtReader<'a>>,
}

impl<'a> ClocksIter<'a> {
    pub fn new(node: &'a Node<'a>) -> Self {
        let fdt = node.fdt.clone();
        let id_list = node.find_property("clocks");
        let name_list = node.find_property("clock-names");

        Self {
            fdt,
            id_list: id_list.map(|p| p.data),
            name_list: name_list.map(|p| p.data),
        }
    }
}

impl<'a> Iterator for ClocksIter<'a> {
    type Item = ClockRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let p = self.id_list.as_mut()?;
        let phandle = Phandle::from(p.take_u32()?);

        let node = self.fdt.get_node_by_phandle(phandle)?;
        let mut select = 0;
        let mut name = None;
        let mut clock_frequency = None;

        let cell_size = node
            .find_property("#clock-cells")
            .expect("#clock-cells not found")
            .u32();

        if cell_size > 0 {
            select = p.take_u32().expect("invalid clock cells");
        } else {
            clock_frequency = node.clock_frequency()
        }

        if let Some(name_prop) = &mut self.name_list {
            name = name_prop.take_str().ok();
        }

        Some(ClockRef {
            node,
            select: select as _,
            name,
            clock_frequency,
        })
    }
}

pub struct ClockRef<'a> {
    pub node: Node<'a>,
    /// second cell of one of `clocks`.
    pub select: usize,
    pub name: Option<&'a str>,
    pub clock_frequency: Option<u32>,
}

impl Debug for ClockRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ClockRef")
            .field("node", &self.node.name())
            .field("select", &self.select)
            .field("name", &self.name)
            .field("clock-frequency", &self.clock_frequency)
            .finish()
    }
}
