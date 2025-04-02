use crate::{node::Node, read::FdtReader, Fdt, Phandle};

pub struct ClocksIter<'a> {
    pub fdt: Fdt<'a>,
    pub prop: Option<FdtReader<'a>>,
}

impl<'a> ClocksIter<'a> {
    pub fn new(node: &'a Node<'a>) -> Self {
        let fdt = node.fdt.clone();
        let prop = node.find_property("clocks");

        Self {
            fdt,
            prop: prop.map(|p| p.data),
        }
    }
}

impl<'a> Iterator for ClocksIter<'a> {
    type Item = ClockRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let p = self.prop.as_mut()?;
        let phandle = Phandle::from(p.take_u32()?);

        let node = self.fdt.get_node_by_phandle(phandle)?;
        let mut select = 0;

        let cell_size = node
            .find_property("#clock-cells")
            .expect("#clock-cells not found")
            .u32();

        if cell_size > 0 {
            select = p.take_u32().expect("invalid clock cells");
            for _ in 0..cell_size - 1 {
                p.take_u32();
            }
        }

        Some(ClockRef {
            node,
            select: select as _,
        })
    }
}

pub struct ClockRef<'a> {
    pub node: Node<'a>,
    /// second cell of one of `clocks`.
    pub select: usize,
}
