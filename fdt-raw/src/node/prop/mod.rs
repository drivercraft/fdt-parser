//! Device tree property types and iterators.
//!
//! This module provides types for representing and iterating over device tree
//! properties, including the generic `Property` type and specialized parsers
//! for common property formats like `reg` and `ranges`.

mod ranges;
mod reg;

use core::ffi::CStr;
use core::fmt;

use log::error;

pub use ranges::*;
pub use reg::{RegInfo, RegIter};

use crate::{
    FdtError, Phandle, Status, Token,
    data::{Bytes, Reader, StrIter, U32Iter},
};

/// A generic device tree property containing name and raw data.
///
/// Represents a property with a name and associated data. Provides methods
/// for accessing and interpreting the data in various formats (u32, u64,
/// strings, etc.).
#[derive(Clone)]
pub struct Property<'a> {
    name: &'a str,
    data: Bytes<'a>,
}

impl<'a> Property<'a> {
    /// Creates a new property from a name and data bytes.
    pub fn new(name: &'a str, data: Bytes<'a>) -> Self {
        Self { name, data }
    }

    /// Returns the property name.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the property data.
    pub fn data(&self) -> Bytes<'a> {
        self.data.clone()
    }

    /// Returns true if the property has no data.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the length of the property data in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns an iterator over u32 values in the property data.
    pub fn as_u32_iter(&self) -> U32Iter<'a> {
        self.data.as_u32_iter()
    }

    /// Returns an iterator over null-terminated strings in the property data.
    ///
    /// Used for properties like `compatible` that contain multiple strings.
    pub fn as_str_iter(&self) -> StrIter<'a> {
        self.data.as_str_iter()
    }

    /// Returns the property data as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Returns the data as a single u64 value.
    ///
    /// Returns None if the data is not exactly 8 bytes.
    pub fn as_u64(&self) -> Option<u64> {
        let mut iter = self.as_u32_iter();
        let high = iter.next()? as u64;
        let low = iter.next()? as u64;
        if iter.next().is_some() {
            return None;
        }
        Some((high << 32) | low)
    }

    /// Returns the data as a single u32 value.
    ///
    /// Returns None if the data is not exactly 4 bytes.
    pub fn as_u32(&self) -> Option<u32> {
        let mut iter = self.as_u32_iter();
        let value = iter.next()?;
        if iter.next().is_some() {
            return None;
        }
        Some(value)
    }

    /// Returns the data as a null-terminated string.
    pub fn as_str(&self) -> Option<&'a str> {
        let bytes = self.data.as_slice();
        let cstr = CStr::from_bytes_until_nul(bytes).ok()?;
        cstr.to_str().ok()
    }

    /// Returns the property value as #address-cells.
    ///
    /// Only returns a value if the property name is "#address-cells".
    pub fn as_address_cells(&self) -> Option<u8> {
        if self.name == "#address-cells" {
            self.as_u32().map(|v| v as u8)
        } else {
            None
        }
    }

    /// Returns the property value as #size-cells.
    ///
    /// Only returns a value if the property name is "#size-cells".
    pub fn as_size_cells(&self) -> Option<u8> {
        if self.name == "#size-cells" {
            self.as_u32().map(|v| v as u8)
        } else {
            None
        }
    }

    /// Returns the property value as #interrupt-cells.
    ///
    /// Only returns a value if the property name is "#interrupt-cells".
    pub fn as_interrupt_cells(&self) -> Option<u8> {
        if self.name == "#interrupt-cells" {
            self.as_u32().map(|v| v as u8)
        } else {
            None
        }
    }

    /// Returns the property value as a Status enum.
    ///
    /// Only returns a value if the property name is "status".
    pub fn as_status(&self) -> Option<Status> {
        let v = self.as_str()?;
        if self.name == "status" {
            match v {
                "okay" | "ok" => Some(Status::Okay),
                "disabled" => Some(Status::Disabled),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Returns the property value as a phandle.
    ///
    /// Only returns a value if the property name is "phandle".
    pub fn as_phandle(&self) -> Option<Phandle> {
        if self.name == "phandle" {
            self.as_u32().map(Phandle::from)
        } else {
            None
        }
    }

    /// Returns the property value as device_type string.
    ///
    /// Only returns a value if the property name is "device_type".
    pub fn as_device_type(&self) -> Option<&'a str> {
        if self.name == "device_type" {
            self.as_str()
        } else {
            None
        }
    }

    /// Returns the property value as interrupt-parent phandle.
    ///
    /// Only returns a value if the property name is "interrupt-parent".
    pub fn as_interrupt_parent(&self) -> Option<Phandle> {
        if self.name == "interrupt-parent" {
            self.as_u32().map(Phandle::from)
        } else {
            None
        }
    }

    /// Returns the property value as clock-names string list.
    ///
    /// Only returns a value if the property name is "clock-names".
    pub fn as_clock_names(&self) -> Option<StrIter<'a>> {
        if self.name == "clock-names" {
            Some(self.as_str_iter())
        } else {
            None
        }
    }

    /// Returns the property value as compatible string list.
    ///
    /// Only returns a value if the property name is "compatible".
    pub fn as_compatible(&self) -> Option<StrIter<'a>> {
        if self.name == "compatible" {
            Some(self.as_str_iter())
        } else {
            None
        }
    }

    /// Returns true if this is a dma-coherent property.
    ///
    /// A dma-coherent property has no data and indicates DMA coherence.
    pub fn is_dma_coherent(&self) -> bool {
        self.name == "dma-coherent" && self.data.is_empty()
    }
}

impl fmt::Display for Property<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            write!(f, "{}", self.name())
        } else if let Some(v) = self.as_address_cells() {
            write!(f, "#address-cells = <{:#x}>", v)
        } else if let Some(v) = self.as_size_cells() {
            write!(f, "#size-cells = <{:#x}>", v)
        } else if let Some(v) = self.as_interrupt_cells() {
            write!(f, "#interrupt-cells = <{:#x}>", v)
        } else if self.name() == "reg" {
            // reg property needs special handling, but we lack context info
            // Display raw data
            write!(f, "reg = ")?;
            format_bytes(f, &self.data())
        } else if let Some(s) = self.as_status() {
            write!(f, "status = \"{:?}\"", s)
        } else if let Some(p) = self.as_phandle() {
            write!(f, "phandle = {}", p)
        } else if let Some(p) = self.as_interrupt_parent() {
            write!(f, "interrupt-parent = {}", p)
        } else if let Some(s) = self.as_device_type() {
            write!(f, "device_type = \"{}\"", s)
        } else if let Some(iter) = self.as_compatible() {
            write!(f, "compatible = ")?;
            let mut first = true;
            for s in iter.clone() {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "\"{}\"", s)?;
                first = false;
            }
            Ok(())
        } else if let Some(iter) = self.as_clock_names() {
            write!(f, "clock-names = ")?;
            let mut first = true;
            for s in iter.clone() {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "\"{}\"", s)?;
                first = false;
            }
            Ok(())
        } else if self.is_dma_coherent() {
            write!(f, "dma-coherent")
        } else if let Some(s) = self.as_str() {
            // Check if there are multiple strings
            if self.data().iter().filter(|&&b| b == 0).count() > 1 {
                write!(f, "{} = ", self.name())?;
                let mut first = true;
                for s in self.as_str_iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\"", s)?;
                    first = false;
                }
                Ok(())
            } else {
                write!(f, "{} = \"{}\"", self.name(), s)
            }
        } else if self.len() == 4 {
            // Single u32
            let v = u32::from_be_bytes(self.data().as_slice().try_into().unwrap());
            write!(f, "{} = <{:#x}>", self.name(), v)
        } else {
            // Raw bytes
            write!(f, "{} = ", self.name())?;
            format_bytes(f, &self.data())
        }
    }
}

/// Formats a byte array as DTS format.
fn format_bytes(f: &mut fmt::Formatter<'_>, data: &[u8]) -> fmt::Result {
    if data.len().is_multiple_of(4) {
        // Format as u32 values
        write!(f, "<")?;
        let mut first = true;
        for chunk in data.chunks(4) {
            if !first {
                write!(f, " ")?;
            }
            let v = u32::from_be_bytes(chunk.try_into().unwrap());
            write!(f, "{:#x}", v)?;
            first = false;
        }
        write!(f, ">")
    } else {
        // Format as bytes
        write!(f, "[")?;
        for (i, b) in data.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{:02x}", b)?;
        }
        write!(f, "]")
    }
}

/// Property iterator.
///
/// Iterates over properties within a node, parsing each property from the
/// device tree structure block.
pub struct PropIter<'a> {
    reader: Reader<'a>,
    strings: Bytes<'a>,
    finished: bool,
}

impl<'a> PropIter<'a> {
    /// Creates a new property iterator.
    pub(crate) fn new(reader: Reader<'a>, strings: Bytes<'a>) -> Self {
        Self {
            reader,
            strings,

            finished: false,
        }
    }

    /// Handles errors: logs error and terminates iteration.
    fn handle_error(&mut self, err: FdtError) {
        error!("Property parse error: {}", err);
        self.finished = true;
    }

    /// Reads a property name from the strings block.
    fn read_prop_name(&self, nameoff: u32) -> Result<&'a str, FdtError> {
        if nameoff as usize >= self.strings.len() {
            return Err(FdtError::BufferTooSmall {
                pos: nameoff as usize,
            });
        }
        let bytes = self.strings.slice(nameoff as usize..self.strings.len());
        let cstr = CStr::from_bytes_until_nul(bytes.as_slice())?;
        Ok(cstr.to_str()?)
    }

    /// Aligns the reader to a 4-byte boundary.
    fn align4(&mut self) {
        let pos = self.reader.position();
        let aligned = (pos + 3) & !3;
        let skip = aligned - pos;
        if skip > 0 {
            let _ = self.reader.read_bytes(skip);
        }
    }
}

impl<'a> Iterator for PropIter<'a> {
    type Item = Property<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            let token = match self.reader.read_token() {
                Ok(t) => t,
                Err(e) => {
                    self.handle_error(e);
                    return None;
                }
            };

            match token {
                Token::Prop => {
                    // Read property length
                    let len = match self.reader.read_u32() {
                        Some(b) => b,
                        None => {
                            self.handle_error(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            });
                            return None;
                        }
                    };

                    // Read property name offset
                    let nameoff = match self.reader.read_u32() {
                        Some(b) => b,
                        None => {
                            self.handle_error(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            });
                            return None;
                        }
                    };

                    // Read property data
                    let prop_data = if len > 0 {
                        match self.reader.read_bytes(len as _) {
                            Some(b) => b,
                            None => {
                                self.handle_error(FdtError::BufferTooSmall {
                                    pos: self.reader.position(),
                                });
                                return None;
                            }
                        }
                    } else {
                        Bytes::new(&[])
                    };

                    // Read property name
                    let name = match self.read_prop_name(nameoff) {
                        Ok(n) => n,
                        Err(e) => {
                            self.handle_error(e);
                            return None;
                        }
                    };

                    // Align to 4-byte boundary
                    self.align4();

                    return Some(Property::new(name, prop_data));
                }
                Token::BeginNode | Token::EndNode | Token::End => {
                    // Encountered node boundary, backtrack and terminate property iteration
                    self.reader.backtrack(4);
                    self.finished = true;
                    return None;
                }
                Token::Nop => {
                    // Ignore NOP and continue
                    continue;
                }
                Token::Data(_) => {
                    // Invalid token
                    self.handle_error(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    });
                    return None;
                }
            }
        }
    }
}
