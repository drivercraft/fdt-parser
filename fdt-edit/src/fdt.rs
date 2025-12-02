use core::ops::Deref;

use alloc::vec::Vec;
use fdt_raw::Token;

use crate::Node;

#[derive(Clone)]
pub struct Fdt {
    pub header: fdt_raw::Header,
    pub nodes: Vec<Node>,
}

impl Fdt {
    pub fn from_ptr(raw_ptr: *const u8) {}

    pub fn to_bytes(&self) -> FdtData {}
}

#[derive(Clone)]
pub struct FdtData(Vec<u32>);

impl FdtData {
    pub fn push(&mut self, value: Token) {
        self.0.push(value.into());
    }
}

impl Deref for FdtData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe {
            core::slice::from_raw_parts(
                self.0.as_ptr() as *const u8,
                self.0.len() * core::mem::size_of::<u32>(),
            )
        }
    }
}
