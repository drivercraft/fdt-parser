//! Device tree property parsing and access.
//!
//! This module provides the `Property` type for accessing device tree
//! property values, with methods for interpreting the data in various formats.

use core::{ffi::CStr, iter};

use crate::{
    base::Fdt,
    data::{Buffer, Raw},
    FdtError, Token,
};

/// A device tree property.
///
/// Properties are key-value pairs associated with device tree nodes.
/// Each property has a name and a value, where the value can be interpreted
/// in various ways depending on the property type.
#[derive(Clone)]
pub struct Property<'a> {
    /// The property name
    pub name: &'a str,
    pub(crate) data: Raw<'a>,
}

impl<'a> Property<'a> {
    /// Returns the raw property value as a byte slice.
    pub fn raw_value(&self) -> &'a [u8] {
        self.data.value()
    }

    /// Interprets the property value as a big-endian u32.
    pub fn u32(&self) -> Result<u32, FdtError> {
        self.data.buffer().take_u32()
    }

    /// Interprets the property value as a big-endian u64.
    pub fn u64(&self) -> Result<u64, FdtError> {
        self.data.buffer().take_u64()
    }

    /// Interprets the property value as a null-terminated string.
    pub fn str(&self) -> Result<&'a str, FdtError> {
        let res = CStr::from_bytes_until_nul(self.data.value())?.to_str()?;
        Ok(res)
    }

    /// Interprets the property value as a list of null-terminated strings.
    pub fn str_list(&self) -> impl Iterator<Item = &'a str> + 'a {
        let mut value = self.data.buffer();
        iter::from_fn(move || value.take_str().ok())
    }

    /// Interprets the property value as a list of big-endian u32 values.
    pub fn u32_list(&self) -> impl Iterator<Item = u32> + 'a {
        let mut value = self.data.buffer();
        iter::from_fn(move || value.take_u32().ok())
    }

    /// Interprets the property value as a list of big-endian u64 values.
    pub fn u64_list(&self) -> impl Iterator<Item = u64> + 'a {
        let mut value = self.data.buffer();
        iter::from_fn(move || value.take_u64().ok())
    }
}

impl core::fmt::Debug for Property<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} = [", self.name)?;
        for v in self.u32_list() {
            write!(f, "{:#x}, ", v)?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

/// Iterator over properties in a device tree node.
pub(crate) struct PropIter<'a> {
    fdt: Fdt<'a>,
    reader: Buffer<'a>,
    has_err: bool,
}

impl<'a> PropIter<'a> {
    pub fn new(fdt: Fdt<'a>, reader: Buffer<'a>) -> Self {
        Self {
            fdt,
            reader,
            has_err: false,
        }
    }

    fn try_next(&mut self) -> Result<Option<Property<'a>>, FdtError> {
        loop {
            match self.reader.take_token()? {
                Token::Prop => break,
                Token::Nop => {}
                _ => return Ok(None),
            }
        }
        let prop = self.reader.take_prop(&self.fdt)?;
        Ok(Some(prop))
    }
}

impl<'a> Iterator for PropIter<'a> {
    type Item = Result<Property<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_err {
            return None;
        }
        match self.try_next() {
            Ok(Some(prop)) => Some(Ok(prop)),
            Ok(None) => None,
            Err(e) => {
                self.has_err = true;
                Some(Err(e))
            }
        }
    }
}
