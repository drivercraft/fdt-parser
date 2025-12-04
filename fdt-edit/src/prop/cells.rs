use core::fmt::Debug;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{PropertyOp, RawProperty, prop::PropertyTrait};

#[derive(Clone)]
pub struct U32(pub(crate) RawProperty);

impl U32 {
    pub fn new(name: &str, num: u32) -> Self {
        let data = num.to_be_bytes();
        let raw = RawProperty::new(name, data.to_vec());
        Self(raw)
    }

    pub fn value(&self) -> u32 {
        let data = self.0.data.as_slice();
        if data.len() != 4 {
            return 0;
        }
        u32::from_be_bytes([data[0], data[1], data[2], data[3]])
    }

    pub fn set_value(&mut self, val: u32) {
        let data = val.to_be_bytes();
        self.0.data = data.to_vec();
    }
}

impl PropertyTrait for U32 {
    fn as_raw(&self) -> &RawProperty {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawProperty {
        &mut self.0
    }
}

impl PropertyOp for U32 {}

impl core::fmt::Debug for U32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} = <{:#x}>", self.0.name, self.value())
    }
}

#[derive(Clone)]
pub struct FStr(pub(crate) RawProperty);

impl FStr {
    pub fn new(name: &str, s: &str) -> Self {
        let mut data = s.as_bytes().to_vec();
        data.push(0);
        let raw = RawProperty::new(name, data);
        Self(raw)
    }

    pub fn value(&self) -> String {
        let data = self.0.data.as_slice();
        String::from_utf8_lossy(data).to_string()
    }

    pub fn set_value(&mut self, s: &str) {
        let mut data = s.as_bytes().to_vec();
        data.push(0);
        self.0.data = data;
    }
}

impl PropertyTrait for FStr {
    fn as_raw(&self) -> &RawProperty {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawProperty {
        &mut self.0
    }
}

impl PropertyOp for FStr {}

impl Debug for FStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} = \"{}\"", self.0.name, self.value())
    }
}

#[derive(Clone)]
pub struct StringList(pub(crate) RawProperty);

impl StringList {
    pub fn new<'a>(name: &str, strings: impl Iterator<Item = &'a str>) -> Self {
        let mut data = Vec::new();
        for s in strings {
            data.extend_from_slice(s.as_bytes());
            data.push(0); // Null terminator
        }
        let raw = RawProperty::new(name, data);
        Self(raw)
    }

    pub fn values(&self) -> Vec<String> {
        let data = self.0.data.as_slice();
        let mut strings = Vec::new();
        let mut start = 0;

        for (i, &byte) in data.iter().enumerate() {
            if byte == 0 {
                if start < i {
                    let s = String::from_utf8_lossy(&data[start..i]).to_string();
                    strings.push(s);
                }
                start = i + 1;
            }
        }

        strings
    }

    pub fn set_values(&mut self, strings: &[&str]) {
        let mut data = Vec::new();
        for s in strings {
            data.extend_from_slice(s.as_bytes());
            data.push(0); // Null terminator
        }
        self.0.data = data;
    }
}

impl Debug for StringList {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let values = self.values();
        write!(f, "{} = [", self.0.name)?;
        for (i, s) in values.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "\"{}\"", s)?;
        }
        write!(f, "]")
    }
}

impl PropertyTrait for StringList {
    fn as_raw(&self) -> &RawProperty {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawProperty {
        &mut self.0
    }
}

impl PropertyOp for StringList {}
