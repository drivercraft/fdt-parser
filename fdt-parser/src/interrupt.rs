use crate::{node::Node, property::Property, read::U32Array};

pub struct InterruptController<'a> {
    pub node: Node<'a>,
}

impl<'a> InterruptController<'a> {
    pub fn interrupt_cells(&self) -> usize {
        self.node.find_property("#interrupt-cells").unwrap().u32() as _
    }
}

pub struct InterruptInfo<'a> {
    pub cell_size: usize,
    pub(crate) prop: Property<'a>,
}

impl<'a> InterruptInfo<'a> {
    pub fn interrupts(&self) -> impl Iterator<Item = u32> + 'a {
        U32Array::new(self.prop.raw_value())
    }
}
