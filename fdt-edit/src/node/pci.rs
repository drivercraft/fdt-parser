use crate::node::{NodeOp, NodeTrait, RawNode};

#[derive(Clone, Debug)]
pub struct NodePci(pub(crate) RawNode);

impl NodeOp for NodePci {}

impl NodeTrait for NodePci {
    fn as_raw(&self) -> &RawNode {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawNode {
        &mut self.0
    }

    fn to_raw(self) -> RawNode {
        self.0
    }
}

impl NodePci {
    pub fn new(name: &str) -> Self {
        NodePci(RawNode::new(name))
    }
}
