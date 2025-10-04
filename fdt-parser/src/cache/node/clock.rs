use core::fmt::Debug;

use crate::{cache::node::NodeBase, FdtError, Phandle};
use alloc::{string::String, string::ToString, vec::Vec};

pub struct ClockInfo {
    pub name: Option<String>,
    pub select: Option<u32>,
    pub clock: ClockType,
}

pub enum ClockType {
    Fixed(FixedClock),
    Other(Clock),
}

impl ClockType {
    pub(super) fn new(node: NodeBase) -> Self {
        todo!()
    }
}


pub struct FixedClock {
    pub frequency: u32,
    pub name: Option<String>,
}

#[derive(Clone)]
pub struct Clock {
    pub node: NodeBase,
}

impl Debug for Clock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Clock")
            .field("name", &self.node.name())
            .finish()
    }
}

impl Clock {
    /// Get the clock provider node name
    pub fn provider_name(&self) -> &str {
        self.node.name()
    }
}
