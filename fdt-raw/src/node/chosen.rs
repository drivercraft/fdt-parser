//! Chosen node type for boot parameters.
//!
//! This module provides the `Chosen` type which represents the /chosen node
//! in the device tree, containing boot parameters and system configuration.

use core::ops::Deref;

use super::NodeBase;

/// The /chosen node containing boot parameters.
///
/// This node contains system configuration parameters chosen by the firmware
/// or bootloader, such as boot arguments, console paths, and other boot-time
/// settings.
#[derive(Clone)]
pub struct Chosen<'a> {
    node: NodeBase<'a>,
}

impl<'a> Chosen<'a> {
    /// Creates a new Chosen wrapper from a NodeBase.
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        Self { node }
    }

    /// Returns the bootargs property value.
    ///
    /// This property contains command-line arguments to be passed to the
    /// operating system kernel.
    pub fn bootargs(&self) -> Option<&'a str> {
        self.node.find_property_str("bootargs")
    }

    /// Returns the stdout-path property value.
    ///
    /// This property specifies the path to the device to be used for
    /// standard output (console).
    pub fn stdout_path(&self) -> Option<&'a str> {
        self.node.find_property_str("stdout-path")
    }

    /// Returns the stdin-path property value.
    ///
    /// This property specifies the path to the device to be used for
    /// standard input.
    pub fn stdin_path(&self) -> Option<&'a str> {
        self.node.find_property_str("stdin-path")
    }
}

impl<'a> Deref for Chosen<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl core::fmt::Debug for Chosen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Chosen")
            .field("bootargs", &self.bootargs())
            .field("stdout_path", &self.stdout_path())
            .finish()
    }
}
