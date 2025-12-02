use core::fmt;

use crate::{FdtError, data::Bytes, header::Header, iter::FdtIter};

#[derive(Clone)]
pub struct Fdt<'a> {
    header: Header,
    pub(crate) data: Bytes<'a>,
}

impl<'a> Fdt<'a> {
    /// Create a new `Fdt` from byte slice.
    pub fn from_bytes(data: &'a [u8]) -> Result<Fdt<'a>, FdtError> {
        let header = Header::from_bytes(data)?;
        if data.len() < header.totalsize as usize {
            return Err(FdtError::BufferTooSmall {
                pos: header.totalsize as usize,
            });
        }
        let buffer = Bytes::new(data);
        Ok(Fdt {
            header,
            data: buffer,
        })
    }

    /// Create a new `Fdt` from a raw pointer and size in bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// memory region of at least `size` bytes that contains a valid device tree
    /// blob.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Fdt<'a>, FdtError> {
        let header = unsafe { Header::from_ptr(ptr)? };

        let data = Bytes::new(unsafe { core::slice::from_raw_parts(ptr, header.totalsize as _) });

        Ok(Fdt { header, data })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn as_slice(&self) -> &'a [u8] {
        self.data.as_slice()
    }

    pub fn all_nodes(&self) -> FdtIter<'a> {
        FdtIter::new(self.clone())
    }
}

impl fmt::Display for Fdt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "/dts-v1/;")?;
        writeln!(f)?;

        let mut prev_level = 0;

        for node in self.all_nodes() {
            let level = node.level();

            // 关闭前一层级的节点
            while prev_level > level {
                prev_level -= 1;
                let indent = "    ".repeat(prev_level);
                writeln!(f, "{}}};\n", indent)?;
            }

            let indent = "    ".repeat(level);
            let name = if node.name().is_empty() {
                "/"
            } else {
                node.name()
            };

            // 打印节点头部
            writeln!(f, "{}{} {{", indent, name)?;

            // 打印属性
            for prop in node.properties() {
                writeln!(f, "{}    {};", indent, prop)?;
            }

            prev_level = level + 1;
        }

        // 关闭剩余的节点
        while prev_level > 0 {
            prev_level -= 1;
            let indent = "    ".repeat(prev_level);
            writeln!(f, "{}}};\n", indent)?;
        }

        Ok(())
    }
}
