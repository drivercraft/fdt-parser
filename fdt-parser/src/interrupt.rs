use crate::node::Node;

#[derive(Clone)]
pub struct InterruptController<'a> {
    pub node: Node<'a>,
}

impl InterruptController<'_> {
    pub fn interrupt_cells(&self) -> usize {
        self.node.find_property("#interrupt-cells").unwrap().u32() as _
    }
}
