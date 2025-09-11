use core::{ffi::CStr, iter};

use crate::data::Raw;

#[derive(Clone)]
pub struct Property<'a> {
    pub name: &'a str,
    pub(crate) data: Raw<'a>,
}

impl<'a> Property<'a> {
    pub fn raw_value(&self) -> &'a [u8] {
        self.data.raw()
    }

    pub fn u32(&self) -> u32 {
        self.data.buffer_at(0).take_u32().unwrap()
    }

    pub fn u64(&self) -> u64 {
        self.data.buffer_at(0).take_u64().unwrap()
    }

    pub fn str(&self) -> &'a str {
        CStr::from_bytes_until_nul(self.data.raw())
            .unwrap()
            .to_str()
            .unwrap()
    }

    pub fn str_list(&self) -> impl Iterator<Item = &'a str> + '_ {
        let mut value = self.data.buffer_at(0);
        iter::from_fn(move || value.take_str().ok())
    }

    pub fn u32_list(&self) -> impl Iterator<Item = u32> + '_ {
        let mut value = self.data.buffer_at(0);
        iter::from_fn(move || value.take_u32().ok())
    }

    pub fn u64_list(&self) -> impl Iterator<Item = u64> + '_ {
        let mut value = self.data.buffer_at(0);
        iter::from_fn(move || value.take_u64().ok())
    }
}
