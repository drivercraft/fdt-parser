//! Cached FDT parser with indexed nodes for efficient lookups.
//!
//! This module provides a cached representation of the device tree that
//! builds an index for fast repeated lookups. It uses more memory than the
//! direct parser but provides O(1) node access by path or phandle.

mod fdt;
mod node;

use core::ops::Deref;

pub use fdt::*;
pub use node::*;

/// A 4-byte aligned buffer for storing FDT data.
///
/// The Device Tree Blob specification requires 4-byte alignment,
/// and this wrapper ensures the allocated memory meets that requirement.
struct Align4Vec {
    ptr: *mut u8,
    size: usize,
}

unsafe impl Send for Align4Vec {}

impl Align4Vec {
    const ALIGN: usize = 4;

    /// Creates a new 4-byte aligned buffer containing the provided data.
    pub fn new(data: &[u8]) -> Self {
        let size = data.len();
        let layout = core::alloc::Layout::from_size_align(size, Self::ALIGN).unwrap();
        let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, size) };
        Align4Vec { ptr, size }
    }
}

impl Drop for Align4Vec {
    /// Deallocates the aligned buffer when dropped.
    fn drop(&mut self) {
        let layout = core::alloc::Layout::from_size_align(self.size, Self::ALIGN).unwrap();
        unsafe { alloc::alloc::dealloc(self.ptr, layout) };
    }
}

impl Deref for Align4Vec {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { alloc::slice::from_raw_parts(self.ptr, self.size) }
    }
}
