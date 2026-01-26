//! Chosen node type for boot parameters.
//!
//! This module provides the `Chosen` type for the /chosen node which contains
//! system configuration parameters passed by the bootloader.

use core::{fmt::Debug, ops::Deref};

use crate::{base::NodeBase, FdtError};

/// Result of debug console lookup.
#[derive(Clone, Debug)]
pub enum DebugCon<'a> {
    /// Found the corresponding device tree node
    Node(NodeBase<'a>),
    /// Found earlycon parameter only in bootargs, with parsed information
    EarlyConInfo {
        /// The name of the early console device (e.g., "uart8250")
        name: &'a str,
        /// The MMIO address of the device
        mmio: u64,
        /// Additional parameters for the early console
        params: Option<&'a str>,
    },
}

/// The /chosen node containing boot parameters.
///
/// The chosen node doesn't represent any actual hardware device but serves
/// as a place to pass parameters to the operating system or bootloader.
#[derive(Clone)]
pub struct Chosen<'a> {
    node: NodeBase<'a>,
}

impl<'a> Chosen<'a> {
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        Chosen { node }
    }

    /// Get the bootargs from the bootargs property, if it exists.
    pub fn bootargs(&self) -> Result<&'a str, FdtError> {
        let prop = self.node.find_property("bootargs")?;
        prop.str()
    }

    /// Get the stdout node specified by the stdout-path property.
    ///
    /// Searches for the node representing stdout, attempting to resolve
    /// aliases if the node name doesn't exist as-is.
    pub fn stdout(&self) -> Result<Stdout<'a>, FdtError> {
        let prop = self.node.find_property("stdout-path")?;

        let path = prop.str()?;

        let mut sp = path.split(':');

        let name = none_ok!(sp.next(), FdtError::NodeNotFound("path"));

        let params = sp.next();
        let node = self
            .node
            .fdt
            .find_nodes(name)
            .next()
            .ok_or(FdtError::NodeNotFound("path"))??;

        Ok(Stdout {
            params,
            node: node.deref().clone(),
        })
    }

    /// Get the debug console information.
    ///
    /// First tries to find the stdout node. If that fails, parses the
    /// bootargs for earlycon configuration.
    pub fn debugcon(&self) -> Result<DebugCon<'a>, FdtError> {
        match self.stdout() {
            Ok(stdout) => Ok(DebugCon::Node(stdout.node.clone())),
            Err(FdtError::NotFound) | Err(FdtError::NodeNotFound(_)) => {
                self.fdt_bootargs_find_debugcon_info()
            }
            Err(e) => Err(e),
        }
    }

    fn fdt_bootargs_find_debugcon_info(&self) -> Result<DebugCon<'a>, FdtError> {
        let bootargs = self.bootargs()?;

        let earlycon = none_ok!(bootargs
            .split_ascii_whitespace()
            .find(|&arg| arg.contains("earlycon")));

        let mut tmp = earlycon.split('=');
        let _ = none_ok!(tmp.next(), FdtError::NotFound);
        let values = none_ok!(tmp.next(), FdtError::NotFound);

        // Parse all parameters
        let mut params_iter = values.split(',');
        let name = none_ok!(params_iter.next(), FdtError::NotFound);

        if !name.contains("uart") {
            return Err(FdtError::NotFound);
        }

        let param2 = none_ok!(params_iter.next(), FdtError::NotFound);

        let addr_str = if param2.contains("0x") {
            param2
        } else {
            none_ok!(params_iter.next(), FdtError::NotFound)
        };

        let mmio = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16)
            .map_err(|_| FdtError::Utf8Parse)?;

        // Try to find the corresponding node in the device tree first
        for node_result in self.node.fdt.all_nodes() {
            let node = node_result?;
            match node.reg() {
                Ok(mut regs) => {
                    for reg in &mut regs {
                        if reg.address == mmio {
                            return Ok(DebugCon::Node(node.node().clone()));
                        }
                    }
                }
                Err(FdtError::NotFound) => {}
                Err(e) => return Err(e),
            }
        }

        // If no matching node is found, return the parsed earlycon information
        // Re-split the string to get remaining parameters
        let mut parts = values.split(',');
        let _name = parts.next(); // skip name
        let _addr_part = parts.next(); // skip address part
        let params = if let Some(param) = parts.next() {
            // Get the position of the first remaining parameter, then take all remaining content
            let param_start = values.find(param).unwrap_or(0);
            if param_start > 0 {
                Some(&values[param_start..])
            } else {
                Some(param)
            }
        } else {
            None
        };

        Ok(DebugCon::EarlyConInfo { name, mmio, params })
    }
}

impl Debug for Chosen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Chosen")
            .field("bootargs", &self.bootargs())
            .field("stdout", &self.stdout())
            .finish()
    }
}

impl<'a> Deref for Chosen<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

/// The stdout device specified by the chosen node.
///
/// Contains the node reference and optional parameters (typically specifying
/// the baud rate or other console configuration).
#[derive(Clone)]
pub struct Stdout<'a> {
    /// Optional parameters for the stdout device (e.g., baud rate)
    pub params: Option<&'a str>,
    /// The device tree node for the stdout device
    pub node: NodeBase<'a>,
}

impl<'a> Stdout<'a> {}

impl Debug for Stdout<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Stdout")
            .field("name", &self.node.name())
            .field("params", &self.params)
            .finish()
    }
}

impl<'a> Deref for Stdout<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
