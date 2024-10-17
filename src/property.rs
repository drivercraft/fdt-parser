use core::ffi::CStr;

use crate::read::FdtReader;

pub struct Property<'a> {
    pub name: &'a str,
    pub(crate) data: FdtReader<'a>,
}

impl<'a> Property<'a> {
    pub fn raw_value(&self) -> &'a [u8] {
        self.data.remaining()
    }

    pub fn u32(&self) -> u32 {
        self.data.clone().take_u32().unwrap()
    }

    pub fn u64(&self) -> u64 {
        self.data.clone().take_u64().unwrap()
    }

    pub fn str(&self) -> &'a str {
        CStr::from_bytes_until_nul(self.data.remaining())
            .unwrap()
            .to_str()
            .unwrap()
    }

}
