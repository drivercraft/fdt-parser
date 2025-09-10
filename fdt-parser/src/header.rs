use core::ptr::NonNull;

use crate::{root::Fdt, FdtError};

#[derive(Debug, Clone)]
pub struct Header {
    /// FDT header magic
    pub magic: u32,
    /// Total size in bytes of the FDT structure
    pub totalsize: u32,
    /// Offset in bytes from the start of the header to the structure block
    pub off_dt_struct: u32,
    /// Offset in bytes from the start of the header to the strings block
    pub off_dt_strings: u32,
    /// Offset in bytes from the start of the header to the memory reservation
    /// block
    pub off_mem_rsvmap: u32,
    /// FDT version
    pub version: u32,
    /// Last compatible FDT version
    pub last_comp_version: u32,
    /// System boot CPU ID
    pub boot_cpuid_phys: u32,
    /// Length in bytes of the strings block
    pub size_dt_strings: u32,
    /// Length in bytes of the struct block
    pub size_dt_struct: u32,
}

impl Header {
    /// Read a header from a raw pointer and return an owned `Header` whose
    /// fields are converted from big-endian (on-disk) to host order.
    pub fn from_ptr(ptr: *mut u8) -> Result<Self, FdtError> {
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
