use core::fmt::Debug;

use crate::{data::Buffer, fdt_no_mem::FdtNoMem, FdtError, Node, NodeBase, Phandle};

pub struct ClocksIter<'a> {
    pub fdt: FdtNoMem<'a>,
    pub id_list: Option<Buffer<'a>>,
    pub name_list: Option<Buffer<'a>>,
    has_error: bool,
}

impl<'a> ClocksIter<'a> {
    pub fn new(node: &NodeBase<'a>) -> Result<Self, FdtError> {
        let fdt = node.fdt.clone();
        let id_list = node.find_property("clocks")?;
        let name_list = node.find_property("clock-names")?;

        Ok(Self {
            fdt,
            id_list: id_list.map(|p| p.data.buffer()),
            name_list: name_list.map(|p| p.data.buffer()),
            has_error: false,
        })
    }
}

impl<'a> Iterator for ClocksIter<'a> {
    type Item = Result<ClockRef<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! bail {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => {
                        self.has_error = true;
                        return Some(Err(e));
                    }
                }
            };
        }

        if self.has_error {
            return None;
        }
        let p = self.id_list.as_mut()?;
        let phandle = p.take_u32().ok()?;

        let phandle = Phandle::from(phandle);

        let node = bail!(self.fdt.get_node_by_phandle(phandle));
        let node = bail!(node.ok_or(FdtError::NodeNotFound("clock phandle")));

        let mut select = 0;
        let mut name = None;
        let mut clock_frequency = None;

        let prop_cell_size = bail!(node.find_property("#clock-cells"));
        let prop_cell_size =
            bail!(prop_cell_size.ok_or(FdtError::PropertyNotFound("#clock-cells")));
        let cell_size = bail!(prop_cell_size.u32());

        if cell_size > 0 {
            select = p.take_u32().expect("invalid clock cells");
        } else {
            clock_frequency = bail!(node.clock_frequency());
        }

        if let Some(name_prop) = &mut self.name_list {
            name = Some(bail!(name_prop.take_str()));
        }

        Some(Ok(ClockRef {
            node,
            select: select as _,
            name,
            clock_frequency,
        }))
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
