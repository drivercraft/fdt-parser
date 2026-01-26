//! Test data and sample Device Tree Blob (DTB) files for the FDT parser.
//!
//! This crate provides embedded DTB files from various hardware platforms
//! for testing purposes, along with a helper struct to ensure 4-byte alignment
//! required by the FDT specification.

use core::ops::Deref;

const TEST_RPI_4_FDT: &[u8] = include_bytes!("dtb/bcm2711-rpi-4-b.dtb");
const TEST_PHYTIUM_FDT: &[u8] = include_bytes!("dtb/phytium.dtb");
const TEST_QEMU_FDT: &[u8] = include_bytes!("dtb/qemu_pci.dtb");
const TEST_3568_FDT: &[u8] = include_bytes!("dtb/rk3568-firefly-roc-pc-se.dtb");
const TEST_RESERVE_FDT: &[u8] = include_bytes!("dtb/test_reserve.dtb");

/// Returns the FDT data for Raspberry Pi 4 Model B.
pub fn fdt_rpi_4b() -> Align4Vec {
    Align4Vec::new(TEST_RPI_4_FDT)
}

/// Returns the FDT data for Phytium platform.
pub fn fdt_phytium() -> Align4Vec {
    Align4Vec::new(TEST_PHYTIUM_FDT)
}

/// Returns the FDT data for QEMU with PCI support.
pub fn fdt_qemu() -> Align4Vec {
    Align4Vec::new(TEST_QEMU_FDT)
}

/// Returns the FDT data for RK3568 Firefly ROC PC SE.
pub fn fdt_3568() -> Align4Vec {
    Align4Vec::new(TEST_3568_FDT)
}

/// Returns the FDT data with reserved memory entries for testing.
pub fn fdt_reserve() -> Align4Vec {
    Align4Vec::new(TEST_RESERVE_FDT)
}

/// A 4-byte aligned buffer for FDT data.
///
/// The Device Tree Blob specification requires that the FDT structure
/// be 4-byte aligned in memory. This wrapper allocates aligned memory
/// and provides raw pointer access for FDT parsing.
pub struct Align4Vec {
    ptr: *mut u8,
    size: usize,
}

impl Align4Vec {
    /// Creates a new 4-byte aligned buffer from the provided data.
    pub fn new(data: &[u8]) -> Self {
        let size = data.len();
        let layout = core::alloc::Layout::from_size_align(size, 4).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, size) };
        Align4Vec { ptr, size }
    }

    /// Returns a raw pointer to the aligned buffer.
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }
}

impl Drop for Align4Vec {
    /// Deallocates the aligned buffer when the `Align4Vec` is dropped.
    fn drop(&mut self) {
        let layout = core::alloc::Layout::from_size_align(self.size, 4).unwrap();
        unsafe { std::alloc::dealloc(self.ptr, layout) };
    }
}

impl Deref for Align4Vec {
    /// Allows treating `Align4Vec` as a byte slice for convenient data access.
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }
}
