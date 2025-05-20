use core::ffi::CStr;

use crate::{
    error::{FdtError, FdtResult},
    property::Property,
    Fdt, Fdt32, Fdt64, FdtReserveEntry, Token,
};

#[derive(Clone)]
pub(crate) struct FdtReader<'a> {
    bytes: &'a [u8],
}

impl<'a> FdtReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn take_u32(&mut self) -> Option<u32> {
        let bytes = self.take(4)?;
        let fdt32: Fdt32 = bytes.into();
        Some(fdt32.get())
    }

    pub fn take_u64(&mut self) -> Option<u64> {
        let bytes = self.take(8)?;
        let fdt64: Fdt64 = bytes.into();
        Some(fdt64.get())
    }

    pub fn take_by_cell_size(&mut self, cell_size: u8) -> Option<u64> {
        match cell_size {
            1 => self.take_u32().map(|s| s as _),
            2 => self.take_u64(),
            _ => panic!("invalid cell size {}", cell_size),
        }
    }
    pub fn skip(&mut self, n_bytes: usize) -> FdtResult<'a> {
        self.bytes = self.bytes.get(n_bytes..).ok_or(FdtError::Eof)?;
        Ok(())
    }

    pub fn remaining(&self) -> &'a [u8] {
        self.bytes
    }

    // pub fn u32(&self) -> FdtResult<u32> {
    //     let bytes = self.bytes.get(..4).ok_or(FdtError::BufferTooSmall)?;
    //     let fdt32: Fdt32 = bytes.into();
    //     Ok(fdt32.get())
    // }

    pub fn take_token(&mut self) -> Option<Token> {
        let u = self.take_u32()?;
        Some(Token::from(u))
    }

    pub fn is_empty(&self) -> bool {
        self.remaining().is_empty()
    }

    pub fn take(&mut self, bytes: usize) -> Option<&'a [u8]> {
        if bytes == 0 {
            return Some(&[]);
        }

        if self.bytes.len() >= bytes {
            let ret = self.bytes.get(..bytes)?;
            let _ = self.skip(bytes);
            return Some(ret);
        }
        None
    }

    // pub fn take_by(&mut self, offset: usize) -> Option<Self> {
    //     let bytes = self.take(offset)?;
    //     Some(FdtReader::new(bytes))
    // }

    pub fn take_aligned(&mut self, len: usize) -> Option<&'a [u8]> {
        let bytes = (len + 3) & !0x3;
        self.take(bytes)
    }

    pub fn skip_4_aligned(&mut self, len: usize) -> FdtResult<'a> {
        self.skip((len + 3) & !0x3)
    }

    pub fn reserved_memory(&mut self) -> Option<FdtReserveEntry> {
        let address = self.take_u64()?;
        let size = self.take_u64()?;
        Some(FdtReserveEntry::new(address, size))
    }

    pub fn take_unit_name(&mut self) -> FdtResult<'a, &'a str> {
        let unit_name = self.peek_str()?;
        let full_name_len = unit_name.len() + 1;
        let _ = self.skip_4_aligned(full_name_len);
        Ok(if unit_name.is_empty() { "/" } else { unit_name })
    }

    pub fn take_prop(&mut self, fdt: &Fdt<'a>) -> Option<Property<'a>> {
        let len = self.take_u32()?;
        let nameoff = self.take_u32()?;
        let bytes = self.take_aligned(len as _)?;
        Some(Property {
            name: fdt.get_str(nameoff as _).unwrap_or("<error>"),
            data: FdtReader { bytes },
        })
    }

    pub fn peek_str(&self) -> FdtResult<'a, &'a str> {
        if self.is_empty() {
            return Err(FdtError::Eof);
        }
        let data = self.remaining();
        let s =
            CStr::from_bytes_until_nul(data).map_err(|_| FdtError::FromBytesUntilNull { data })?;
        s.to_str().map_err(|_| FdtError::Utf8Parse { data })
    }

    pub fn take_str(&mut self) -> FdtResult<'a, &'a str> {
        let s = self.peek_str()?;
        let _ = self.skip(s.len() + 1);
        Ok(s)
    }
}

pub struct U32Array<'a> {
    reader: FdtReader<'a>,
}

impl<'a> U32Array<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            reader: FdtReader::new(bytes),
        }
    }

    pub fn as_u64(&mut self) -> u64 {
        let h = self.reader.take_u32().unwrap();
        if let Some(l) = self.reader.take_u32() {
            ((h as u64) << 32) + l as u64
        } else {
            h as _
        }
    }
}

impl Iterator for U32Array<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        self.reader.take_u32()
    }
}

pub struct U32Array2D<'a> {
    reader: FdtReader<'a>,
    row_len: usize,
}

impl<'a> U32Array2D<'a> {
    pub fn new(bytes: &'a [u8], row_len: usize) -> Self {
        Self {
            reader: FdtReader::new(bytes),
            row_len,
        }
    }
}

impl<'a> Iterator for U32Array2D<'a> {
    type Item = U32Array<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.reader.take(self.row_len * size_of::<u32>())?;
        Some(U32Array::new(bytes))
    }
}
