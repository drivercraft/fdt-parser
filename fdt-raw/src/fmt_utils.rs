//! Formatting utilities for FDT display output.
//!
//! This module provides helper functions for formatting device tree
//! structures with proper indentation.

use core::fmt;

/// Writes indentation to a formatter.
///
/// Repeats the specified character `count` times, used for
/// indentation when displaying device tree structures.
///
/// # Arguments
///
/// * `f` - The formatter to write to
/// * `count` - Number of times to repeat the character
/// * `ch` - The character to use for indentation
pub fn write_indent(f: &mut fmt::Formatter<'_>, count: usize, ch: &str) -> fmt::Result {
    for _ in 0..count {
        write!(f, "{}", ch)?;
    }
    Ok(())
}
