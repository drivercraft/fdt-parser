//! FDT header parsing.
//!
//! This module handles parsing of the Flattened Device Tree header structure,
//! which appears at the beginning of every device tree blob and contains
//! metadata about the layout and version of the FDT.

use core::ptr::NonNull;

use crate::FdtError;

/// A 4-byte aligned buffer for header data.
///
/// The Device Tree Blob specification requires 4-byte alignment, and this
/// wrapper ensures that we have properly aligned memory when reading from
/// potentially unaligned pointers.
#[repr(align(4))]
struct AlignedHeader([u8; size_of::<Header>()]);

/// The FDT header structure.
///
/// Every device tree blob begins with this header, which contains metadata
/// about the layout and version of the FDT. All fields are stored in big-endian
/// byte order on-disk and are converted to host byte order when parsed.
#[derive(Debug, Clone)]
pub struct Header {
    /// FDT header magic number (must be 0xd00dfeed)
    pub magic: u32,
    /// Total size in bytes of the FDT structure
    pub totalsize: u32,
    /// Offset in bytes from the start of the header to the structure block
    pub off_dt_struct: u32,
    /// Offset in bytes from the start of the header to the strings block
    pub off_dt_strings: u32,
    /// Offset in bytes from the start of the header to the memory reservation block
    pub off_mem_rsvmap: u32,
    /// FDT version number
    pub version: u32,
    /// Last compatible FDT version
    pub last_comp_version: u32,
    /// Physical ID of the boot CPU
    pub boot_cpuid_phys: u32,
    /// Length in bytes of the strings block
    pub size_dt_strings: u32,
    /// Length in bytes of the structure block
    pub size_dt_struct: u32,
}

impl Header {
    /// Read a header from a byte slice.
    ///
    /// Parses an FDT header from the beginning of a byte slice, validating
    /// the magic number and converting all fields from big-endian to host order.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::BufferTooSmall` if the slice is too small to contain
    /// a complete header, or `FdtError::InvalidMagic` if the magic number doesn't
    /// match the expected value.
    pub fn from_bytes(data: &[u8]) -> Result<Self, FdtError> {
        if data.len() < core::mem::size_of::<Header>() {
            return Err(FdtError::BufferTooSmall {
                pos: core::mem::size_of::<Header>(),
            });
        }
        let ptr = NonNull::new(data.as_ptr() as *mut u8).ok_or(FdtError::InvalidPtr)?;
        unsafe { Self::from_ptr(ptr.as_ptr()) }
    }

    /// Read a header from a raw pointer.
    ///
    /// Parses an FDT header from the memory location pointed to by `ptr`,
    /// validating the magic number and converting all fields from big-endian
    /// to host order. Handles unaligned pointers by copying to an aligned buffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// memory region of at least `size_of::<Header>()` bytes that contains a
    /// valid device tree blob header.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::InvalidPtr` if the pointer is null, or
    /// `FdtError::InvalidMagic` if the magic number doesn't match.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Self, FdtError> {
        if !(ptr as usize).is_multiple_of(core::mem::align_of::<Header>()) {
            // Pointer is not aligned, so we need to copy the data to an aligned
            // buffer first.
            let mut aligned = AlignedHeader([0u8; core::mem::size_of::<Header>()]);
            unsafe {
                core::ptr::copy_nonoverlapping(
                    ptr,
                    aligned.0.as_mut_ptr(),
                    core::mem::size_of::<Header>(),
                );
            }
            Self::from_aligned_ptr(aligned.0.as_mut_ptr())
        } else {
            // Pointer is aligned, we can read directly from it.
            Self::from_aligned_ptr(ptr)
        }
    }

    /// Read a header from an aligned pointer.
    ///
    /// Internal helper that assumes the pointer is already 4-byte aligned.
    /// Reads the raw header bytes and converts each field from big-endian.
    ///
    /// # Safety
    ///
    /// Caller must ensure the pointer is valid, aligned, and points to
    /// sufficient memory containing a valid FDT header.
    fn from_aligned_ptr(ptr: *mut u8) -> Result<Self, FdtError> {
        let ptr = NonNull::new(ptr).ok_or(FdtError::InvalidPtr)?;

        // SAFETY: caller provided a valid pointer to the beginning of a device
        // tree blob. We read the raw header as it exists in memory (which is
        // big-endian on-disk). Then convert each u32 field from big-endian to
        // host order using `u32::from_be`.
        let raw = unsafe { &*(ptr.cast::<Header>().as_ptr()) };

        let magic = u32::from_be(raw.magic);
        if magic != crate::FDT_MAGIC {
            return Err(FdtError::InvalidMagic(magic));
        }

        Ok(Header {
            magic,
            totalsize: u32::from_be(raw.totalsize),
            off_dt_struct: u32::from_be(raw.off_dt_struct),
            off_dt_strings: u32::from_be(raw.off_dt_strings),
            off_mem_rsvmap: u32::from_be(raw.off_mem_rsvmap),
            version: u32::from_be(raw.version),
            last_comp_version: u32::from_be(raw.last_comp_version),
            boot_cpuid_phys: u32::from_be(raw.boot_cpuid_phys),
            size_dt_strings: u32::from_be(raw.size_dt_strings),
            size_dt_struct: u32::from_be(raw.size_dt_struct),
        })
    }
}
