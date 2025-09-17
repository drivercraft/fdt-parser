use core::{ffi::CStr, iter};

use crate::{
    data::{Buffer, Raw},
    Fdt, Token,
};

#[derive(Clone)]
pub struct Property<'a> {
    pub name: &'a str,
    pub(crate) data: Raw<'a>,
}

impl<'a> Property<'a> {
    pub fn raw_value(&self) -> &'a [u8] {
        self.data.value()
    }

    pub fn u32(&self) -> u32 {
        self.data.buffer().take_u32().unwrap()
    }

    pub fn u64(&self) -> u64 {
        self.data.buffer().take_u64().unwrap()
    }

    pub fn str(&self) -> &'a str {
        CStr::from_bytes_until_nul(self.data.value())
            .unwrap()
            .to_str()
            .unwrap()
    }

    pub fn str_list(&self) -> impl Iterator<Item = &'a str> + '_ {
        let mut value = self.data.buffer();
        iter::from_fn(move || value.take_str().ok())
    }

    pub fn u32_list(&self) -> impl Iterator<Item = u32> + '_ {
        let mut value = self.data.buffer();
        iter::from_fn(move || value.take_u32().ok())
    }

    pub fn u64_list(&self) -> impl Iterator<Item = u64> + '_ {
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

pub(crate) struct PropIter<'a> {
    pub fdt: Fdt<'a>,
    pub reader: Buffer<'a>,
}

impl<'a> Iterator for PropIter<'a> {
    type Item = Property<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.reader.take_token().ok() {
                Some(token) => match token {
                    Token::Prop => break,
                    Token::Nop => {}
                    _ => return None,
                },
                None => return None,
            }
        }
        self.reader.take_prop(&self.fdt)
    }
}
