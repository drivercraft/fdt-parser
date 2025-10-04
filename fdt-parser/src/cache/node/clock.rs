use core::ops::Deref;

use crate::{cache::node::NodeBase, Phandle};
use alloc::{string::String, string::ToString, vec::Vec};

#[derive(Clone, Debug)]
pub struct ClockInfo {
    /// Name supplied by the consumer through `clock-names`
    pub name: Option<String>,
    /// Name exposed by the provider via `clock-output-names` that matches the specifier
    pub provider_output_name: Option<String>,

    pub phandle: Phandle,
    pub select: u64,
    /// Provider details
    pub provider: ClockType,
}

impl ClockInfo {
    /// Helper access to the provider node
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Number of cells defined by the provider for each specifier
    pub fn provider_clock_cells(&self) -> u32 {
        self.provider.clock_cells()
    }
}

#[derive(Clone, Debug)]
pub enum ClockType {
    Fixed(FixedClock),
    Provider(Clock),
}

impl ClockType {
    pub(super) fn new(node: NodeBase) -> Self {
        let base = Clock::from_node(node.clone());
        let compatibles = node.compatibles();
        if compatibles.iter().any(|c| c == "fixed-clock") {
            ClockType::Fixed(FixedClock {
                clock: base,
                frequency: node
                    .find_property("clock-frequency")
                    .and_then(|p| p.u32().ok()),
                accuracy: node
                    .find_property("clock-accuracy")
                    .and_then(|p| p.u32().ok()),
            })
        } else {
            ClockType::Provider(base)
        }
    }

    pub fn clock_cells(&self) -> u32 {
        match self {
            ClockType::Fixed(fixed) => fixed.clock.clock_cells,
            ClockType::Provider(clock) => clock.clock_cells,
        }
    }

    pub fn output_name(&self, select: u64) -> Option<String> {
        match self {
            ClockType::Fixed(fixed) => fixed.clock.output_name(select),
            ClockType::Provider(clock) => clock.output_name(select),
        }
    }
}

impl Deref for ClockType {
    type Target = NodeBase;

    fn deref(&self) -> &Self::Target {
        match self {
            ClockType::Fixed(fixed) => &fixed.clock.node,
            ClockType::Provider(clock) => &clock.node,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FixedClock {
    pub clock: Clock,
    pub frequency: Option<u32>,
    pub accuracy: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Clock {
    pub node: NodeBase,
    pub clock_cells: u32,
    pub output_names: Vec<String>,
}

impl Clock {
    pub(crate) fn from_node(node: NodeBase) -> Self {
        let clock_cells = node
            .find_property("#clock-cells")
            .and_then(|p| p.u32().ok())
            .unwrap_or(0);
        let output_names = node
            .find_property("clock-output-names")
            .map(|p| p.str_list().map(|s| s.to_string()).collect())
            .unwrap_or_else(Vec::new);

        Self {
            node,
            clock_cells,
            output_names,
        }
    }

    pub fn output_name(&self, select: u64) -> Option<String> {
        if self.output_names.is_empty() {
            return None;
        }

        if self.clock_cells == 0 {
            return self.output_names.first().cloned();
        }

        let index = select as usize;
        self.output_names.get(index).cloned()
    }
}
