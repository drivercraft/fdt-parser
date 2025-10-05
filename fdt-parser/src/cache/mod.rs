mod fdt;
mod node;

use core::ops::Deref;

pub use fdt::*;
pub use node::*;

struct Align4Vec {
    ptr: *mut u8,
    size: usize,
}

unsafe impl Send for Align4Vec {}

impl Align4Vec {
    const ALIGN: usize = 4;

    pub fn new(data: &[u8]) -> Self {
        let size = data.len();
        let layout = core::alloc::Layout::from_size_align(size, Self::ALIGN).unwrap();
        let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, size) };
        Align4Vec { ptr, size }
    }
}

impl Drop for Align4Vec {
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
