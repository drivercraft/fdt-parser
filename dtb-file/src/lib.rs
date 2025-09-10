use core::ops::Deref;

const TEST_RPI_4_FDT: &[u8] = include_bytes!("dtb/bcm2711-rpi-4-b.dtb");
const TEST_PHYTIUM_FDT: &[u8] = include_bytes!("dtb/phytium.dtb");
const TEST_QEMU_FDT: &[u8] = include_bytes!("dtb/qemu_pci.dtb");
const TEST_3568_FDT: &[u8] = include_bytes!("dtb/rk3568-firefly-roc-pc-se.dtb");

pub fn fdt_rpi_4b() -> Align4Vec {
    Align4Vec::new(TEST_RPI_4_FDT)
}

pub fn fdt_phytium() -> Align4Vec {
    Align4Vec::new(TEST_PHYTIUM_FDT)
}

pub fn fdt_qemu() -> Align4Vec {
    Align4Vec::new(TEST_QEMU_FDT)
}

pub fn fdt_3568() -> Align4Vec {
    Align4Vec::new(TEST_3568_FDT)
}

pub struct Align4Vec {
    ptr: *mut u8,
    size: usize,
}

impl Align4Vec {
    pub fn new(data: &[u8]) -> Self {
        let size = data.len();
        let layout = core::alloc::Layout::from_size_align(size, 4).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, size) };
        Align4Vec { ptr, size }
    }
}

impl Drop for Align4Vec {
    fn drop(&mut self) {
        let layout = core::alloc::Layout::from_size_align(self.size, 4).unwrap();
        unsafe { std::alloc::dealloc(self.ptr, layout) };
    }
}

impl Deref for Align4Vec {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }
}
