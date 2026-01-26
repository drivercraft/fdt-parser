use core::ops::Deref;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use fdt_raw::Phandle;

use crate::node::gerneric::NodeRefGen;

/// Clock provider type
#[derive(Clone, Debug, PartialEq)]
pub enum ClockType {
    /// Fixed clock
    Fixed(FixedClock),
    /// Normal clock provider
    Normal,
}

/// Fixed clock provider.
///
/// Represents a fixed-rate clock that always operates at a constant frequency.
#[derive(Clone, Debug, PartialEq)]
pub struct FixedClock {
    /// Optional name for the clock
    pub name: Option<String>,
    /// Clock frequency in Hz
    pub frequency: u32,
    /// Clock accuracy in ppb (parts per billion)
    pub accuracy: Option<u32>,
}

/// Clock reference, used to parse clocks property
///
/// According to the device tree specification, the clocks property format is:
/// `clocks = <&clock_provider specifier [specifier ...]> [<&clock_provider2 ...>]`
///
/// Each clock reference consists of a phandle and several specifier cells,
/// the number of specifiers is determined by the target clock provider's `#clock-cells` property.
#[derive(Clone, Debug)]
pub struct ClockRef {
    /// Clock name, from clock-names property
    pub name: Option<String>,
    /// Phandle of the clock provider
    pub phandle: Phandle,
    /// #clock-cells value of the provider
    pub cells: u32,
    /// Clock selector (specifier), usually the first value is used to select clock output
    /// Length is determined by provider's #clock-cells
    pub specifier: Vec<u32>,
}

impl ClockRef {
    /// Create a new clock reference
    pub fn new(phandle: Phandle, cells: u32, specifier: Vec<u32>) -> Self {
        Self {
            name: None,
            phandle,
            cells,
            specifier,
        }
    }

    /// Create a named clock reference
    pub fn with_name(
        name: Option<String>,
        phandle: Phandle,
        cells: u32,
        specifier: Vec<u32>,
    ) -> Self {
        Self {
            name,
            phandle,
            cells,
            specifier,
        }
    }

    /// Get the first value of the selector (usually used to select clock output)
    ///
    /// Only returns a selector value when `cells > 0`,
    /// because providers with `#clock-cells = 0` don't need a selector.
    pub fn select(&self) -> Option<u32> {
        if self.cells > 0 {
            self.specifier.first().copied()
        } else {
            None
        }
    }
}

/// Clock provider node reference.
///
/// Provides specialized access to clock provider nodes and their properties.
#[derive(Clone)]
pub struct NodeRefClock<'a> {
    /// The underlying generic node reference
    pub node: NodeRefGen<'a>,
    /// Names of clock outputs from this provider
    pub clock_output_names: Vec<String>,
    /// Value of the `#clock-cells` property
    pub clock_cells: u32,
    /// The type of clock provider
    pub kind: ClockType,
}

impl<'a> NodeRefClock<'a> {
    /// Attempts to create a clock provider reference from a generic node.
    ///
    /// Returns `Err` with the original node if it doesn't have a `#clock-cells` property.
    pub fn try_from(node: NodeRefGen<'a>) -> Result<Self, NodeRefGen<'a>> {
        // Check if it has clock provider properties
        if node.find_property("#clock-cells").is_none() {
            return Err(node);
        }

        // Get clock-output-names property
        let clock_output_names = if let Some(prop) = node.find_property("clock-output-names") {
            let iter = prop.as_str_iter();
            iter.map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        // Get #clock-cells
        let clock_cells = node
            .find_property("#clock-cells")
            .and_then(|prop| prop.get_u32())
            .unwrap_or(0);

        // Determine clock type
        let kind = if node.compatibles().any(|c| c == "fixed-clock") {
            let frequency = node
                .find_property("clock-frequency")
                .and_then(|prop| prop.get_u32())
                .unwrap_or(0);
            let accuracy = node
                .find_property("clock-accuracy")
                .and_then(|prop| prop.get_u32());
            let name = clock_output_names.first().cloned();

            ClockType::Fixed(FixedClock {
                name,
                frequency,
                accuracy,
            })
        } else {
            ClockType::Normal
        };

        Ok(Self {
            node,
            clock_output_names,
            clock_cells,
            kind,
        })
    }

    /// Get clock output name (for provider)
    pub fn output_name(&self, index: usize) -> Option<&str> {
        self.clock_output_names.get(index).map(|s| s.as_str())
    }

    /// Parse clocks property, return list of clock references
    ///
    /// By looking up each phandle's corresponding clock provider's #clock-cells,
    /// correctly parse the specifier length.
    pub fn clocks(&self) -> Vec<ClockRef> {
        let Some(prop) = self.find_property("clocks") else {
            return Vec::new();
        };

        let mut clocks = Vec::new();
        let mut data = prop.as_reader();
        let mut index = 0;

        // Get clock-names for naming
        let clock_names = if let Some(prop) = self.find_property("clock-names") {
            let iter = prop.as_str_iter();
            iter.map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        while let Some(phandle_raw) = data.read_u32() {
            let phandle = Phandle::from(phandle_raw);

            // Look up provider node by phandle, get its #clock-cells
            let clock_cells = if let Some(provider) = self.ctx.find_by_phandle(phandle) {
                provider
                    .get_property("#clock-cells")
                    .and_then(|p| p.get_u32())
                    .unwrap_or(1) // Default 1 cell
            } else {
                1 // Default 1 cell
            };

            // Read specifier (based on provider's #clock-cells)
            let mut specifier = Vec::with_capacity(clock_cells as usize);
            let mut complete = true;
            for _ in 0..clock_cells {
                if let Some(val) = data.read_u32() {
                    specifier.push(val);
                } else {
                    // Insufficient data, stop parsing
                    complete = false;
                    break;
                }
            }

            // Only add complete clock reference
            if !complete {
                break;
            }

            // Get corresponding name from clock-names
            let name = clock_names.get(index).cloned();

            clocks.push(ClockRef::with_name(name, phandle, clock_cells, specifier));
            index += 1;
        }

        clocks
    }
}

impl<'a> Deref for NodeRefClock<'a> {
    type Target = NodeRefGen<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
