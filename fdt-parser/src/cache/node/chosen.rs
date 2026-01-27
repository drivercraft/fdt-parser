use core::{fmt::Debug, ops::Deref};

use crate::cache::node::NodeBase;
use alloc::{string::String, string::ToString};

/// The /chosen node containing boot parameters (cached version).
///
/// The chosen node doesn't represent any actual hardware device but serves
/// as a place to pass parameters to the operating system or bootloader.
#[derive(Clone)]
pub struct Chosen {
    node: NodeBase,
}

impl Chosen {
    pub(crate) fn new(node: NodeBase) -> Self {
        Chosen { node }
    }

    /// Contains the bootargs, if they exist
    pub fn bootargs(&self) -> Option<String> {
        self.node
            .find_property("bootargs")
            .and_then(|prop| prop.str().ok())
            .map(|s| s.to_string())
    }

    /// Searches for the node representing `stdout`, if the property exists,
    /// attempting to resolve aliases if the node name doesn't exist as-is
    pub fn stdout(&self) -> Option<Stdout> {
        let prop = self.node.find_property("stdout-path")?;
        let path = prop.str().ok()?;

        let mut sp = path.split(':');
        let name = sp.next()?;
        let params = sp.next();

        // Try to find the node in the cache
        self.node.fdt.get_node_by_path(name).map(|node| Stdout {
            params: params.map(|s| s.to_string()),
            node,
        })
    }

    /// Get the debug console information.
    ///
    /// First tries to find the stdout node. If that fails, parses the
    /// bootargs for earlycon configuration.
    pub fn debugcon(&self) -> Option<DebugConCache> {
        if let Some(stdout) = self.stdout() {
            Some(DebugConCache::Node(stdout.node))
        } else {
            self.fdt_bootargs_find_debugcon_info()
        }
    }

    fn fdt_bootargs_find_debugcon_info(&self) -> Option<DebugConCache> {
        let bootargs = self.bootargs()?;

        // Look for earlycon parameter
        let earlycon = bootargs
            .split_ascii_whitespace()
            .find(|arg| arg.contains("earlycon"))?;

        let mut tmp = earlycon.split('=');
        let _ = tmp.next()?; // skip "earlycon"
        let values = tmp.next()?;

        // Parse all parameters
        let mut params_iter = values.split(',');
        let name = params_iter.next()?;

        if !name.contains("uart") {
            return None;
        }

        let param2 = params_iter.next()?;

        let addr_str = if param2.contains("0x") {
            param2
        } else {
            params_iter.next()?
        };

        let mmio = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16).ok()?;

        // Try to find the corresponding node in the cache first
        let all_nodes = self.node.fdt.all_nodes();
        for node in all_nodes {
            let Ok(reg) = node.reg() else {
                continue;
            };

            for address in reg {
                if address.address == mmio {
                    return Some(DebugConCache::Node(node));
                }
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
                Some(values[param_start..].to_string())
            } else {
                Some(param.to_string())
            }
        } else {
            None
        };

        Some(DebugConCache::EarlyConInfo {
            name: name.to_string(),
            mmio,
            params,
        })
    }
}

impl Debug for Chosen {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Chosen")
            .field("bootargs", &self.bootargs())
            .field("stdout", &self.stdout())
            .finish()
    }
}

impl Deref for Chosen {
    type Target = NodeBase;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

/// Result of debug console lookup for the cached parser.
#[derive(Clone, Debug)]
pub enum DebugConCache {
    /// Found the corresponding device tree node
    Node(super::super::Node),
    /// Found earlycon parameter only in bootargs, with parsed information
    EarlyConInfo {
        /// The name of the early console device (e.g., "uart8250")
        name: String,
        /// The MMIO address of the device
        mmio: u64,
        /// Additional parameters for the early console
        params: Option<String>,
    },
}

/// The stdout device specified by the chosen node (cached version).
///
/// Contains the node reference and optional parameters (typically specifying
/// the baud rate or other console configuration).
#[derive(Clone)]
pub struct Stdout {
    /// Optional parameters for the stdout device (e.g., baud rate)
    pub params: Option<String>,
    /// The device tree node for the stdout device
    pub node: super::super::Node,
}

impl Stdout {}

impl Debug for Stdout {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Stdout")
            .field("name", &self.node.name())
            .field("params", &self.params)
            .finish()
    }
}
