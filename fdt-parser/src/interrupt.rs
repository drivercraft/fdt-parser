use crate::{node::Node, property::Property};

pub struct InterruptController<'a> {
    pub node: Node<'a>,
}

impl<'a> InterruptController<'a> {
    pub fn interrupt_cells(&self) -> usize {
        self.node.find_property("#interrupt-cells").unwrap().u32() as _
    }
}

pub struct InterruptList<'a> {
    pub size_cell: u8,
    pub prop: Property<'a>,
    pub node: Node<'a>,
}

pub struct Interrupt<'a> {
    pub size_cell: u8,
    pub prop: Property<'a>,
    pub node: Node<'a>,
}
