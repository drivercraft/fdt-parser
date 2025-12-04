use crate::node::{NodeTrait, RawNode};

#[derive(Clone, Debug)]
pub struct NodePci(RawNode);

impl NodeTrait for NodePci {
    fn as_raw(&self) -> &RawNode {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawNode {
        &mut self.0
    }
}
