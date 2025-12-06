use core::ffi::CStr;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use fdt_raw::data::{Bytes, Reader, StrIter, U32Iter};
// Re-export from fdt_raw
pub use fdt_raw::{Phandle, RegInfo, Status};

#[derive(Clone)]
pub struct Property {
    pub name: String,
    pub data: Vec<u8>,
}

impl Property {
    pub fn new(name: &str, data: Vec<u8>) -> Self {
        Self {
            name: name.to_string(),
            data,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_u32(&self) -> Option<u32> {
        if self.data.len() != 4 {
            return None;
        }
        Some(u32::from_be_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
        ]))
    }

    pub fn set_u32_ls(&mut self, values: &[u32]) {
        self.data.clear();
        for &value in values {
            self.data.extend_from_slice(&value.to_be_bytes());
        }
    }

    pub fn get_u32_iter(&self) -> U32Iter<'_> {
        Bytes::new(&self.data).as_u32_iter()
    }

    pub fn get_u64(&self) -> Option<u64> {
        if self.data.len() != 8 {
            return None;
        }
        Some(u64::from_be_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
        ]))
    }

    pub fn set_u64(&mut self, value: u64) {
        self.data = value.to_be_bytes().to_vec();
    }

    pub fn as_str(&self) -> Option<&str> {
        CStr::from_bytes_with_nul(&self.data)
            .ok()
            .and_then(|cstr| cstr.to_str().ok())
    }

    pub fn set_string(&mut self, value: &str) {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0); // Null-terminate
        self.data = bytes;
    }

    pub fn as_str_iter(&self) -> StrIter<'_> {
        Bytes::new(&self.data).as_str_iter()
    }

    pub fn set_string_ls(&mut self, values: &[&str]) {
        self.data.clear();
        for &value in values {
            self.data.extend_from_slice(value.as_bytes());
            self.data.push(0); // Null-terminate each string
        }
    }

    pub fn as_reader(&self) -> Reader<'_> {
        Bytes::new(&self.data).reader()
    }
}

impl From<&fdt_raw::Property<'_>> for Property {
    fn from(value: &fdt_raw::Property<'_>) -> Self {
        Self {
            name: value.name().to_string(),
            data: value.as_slice().to_vec(),
        }
    }
}

/// Ranges 条目信息
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangesEntry {
    /// 子总线地址
    pub child_bus_address: u64,
    /// 父总线地址
    pub parent_bus_address: u64,
    /// 区域长度
    pub length: u64,
}
