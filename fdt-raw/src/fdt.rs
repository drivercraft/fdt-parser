use core::fmt;

use crate::{FdtError, data::Bytes, header::Header, iter::FdtIter};

/// 写入缩进（使用空格）
fn write_indent(f: &mut fmt::Formatter<'_>, count: usize, ch: &str) -> fmt::Result {
    for _ in 0..count {
        write!(f, "{}", ch)?;
    }
    Ok(())
}

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
                write_indent(f, prev_level, "    ")?;
                writeln!(f, "}};\n")?;
            }

            write_indent(f, level, "    ")?;
            let name = if node.name().is_empty() {
                "/"
            } else {
                node.name()
            };

            // 打印节点头部
            writeln!(f, "{} {{", name)?;

            // 打印属性
            for prop in node.properties() {
                write_indent(f, level + 1, "    ")?;
                writeln!(f, "{};", prop)?;
            }

            prev_level = level + 1;
        }

        // 关闭剩余的节点
        while prev_level > 0 {
            prev_level -= 1;
            write_indent(f, prev_level, "    ")?;
            writeln!(f, "}};\n")?;
        }

        Ok(())
    }
}

impl fmt::Debug for Fdt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Fdt {{")?;
        writeln!(f, "\theader: {:?}", self.header)?;
        writeln!(f, "\tnodes:")?;

        for node in self.all_nodes() {
            let level = node.level();
            // 基础缩进 2 个 tab，每层再加 1 个 tab
            write_indent(f, level + 2, "\t")?;

            let name = if node.name().is_empty() {
                "/"
            } else {
                node.name()
            };

            // 打印节点名称和基本信息
            writeln!(
                f,
                "[{}] address_cells={}, size_cells={}",
                name, node.address_cells, node.size_cells
            )?;

            // 打印属性
            for prop in node.properties() {
                write_indent(f, level + 3, "\t")?;
                // 使用 Debug 格式展示解析后的属性
                match &prop {
                    crate::Property::Reg(reg) => {
                        write!(f, "reg: [")?;
                        for (i, info) in reg.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{{addr: {:#x}, size: {:?}}}", info.address, info.size)?;
                        }
                        writeln!(f, "]")?;
                    }

                    crate::Property::Compatible(iter) => {
                        write!(f, "compatible: [")?;
                        for (i, s) in iter.clone().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "\"{}\"", s)?;
                        }
                        writeln!(f, "]")?;
                    }
                    crate::Property::AddressCells(v) => {
                        writeln!(f, "#address-cells: {}", v)?;
                    }
                    crate::Property::SizeCells(v) => {
                        writeln!(f, "#size-cells: {}", v)?;
                    }
                    crate::Property::InterruptCells(v) => {
                        writeln!(f, "#interrupt-cells: {}", v)?;
                    }
                    crate::Property::Model(s) => {
                        writeln!(f, "model: \"{}\"", s)?;
                    }
                    crate::Property::DeviceType(s) => {
                        writeln!(f, "device_type: \"{}\"", s)?;
                    }
                    crate::Property::Status(s) => {
                        writeln!(f, "status: {:?}", s)?;
                    }
                    crate::Property::Phandle(p) => {
                        writeln!(f, "phandle: {}", p)?;
                    }
                    crate::Property::LinuxPhandle(p) => {
                        writeln!(f, "linux,phandle: {}", p)?;
                    }
                    crate::Property::InterruptParent(p) => {
                        writeln!(f, "interrupt-parent: {}", p)?;
                    }

                    crate::Property::ClockNames(iter) => {
                        write!(f, "clock-names: [")?;
                        for (i, s) in iter.clone().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "\"{}\"", s)?;
                        }
                        writeln!(f, "]")?;
                    }
                    crate::Property::DmaCoherent => {
                        writeln!(f, "dma-coherent")?;
                    }
                    crate::Property::Unknown(raw) => {
                        if raw.is_empty() {
                            writeln!(f, "{}", raw.name())?;
                        } else if let Some(s) = raw.as_str() {
                            writeln!(f, "{}: \"{}\"", raw.name(), s)?;
                        } else if raw.len() == 4 {
                            let v = u32::from_be_bytes(raw.data().try_into().unwrap());
                            writeln!(f, "{}: {:#x}", raw.name(), v)?;
                        } else {
                            writeln!(f, "{}: <{} bytes>", raw.name(), raw.len())?;
                        }
                    }
                }
            }
        }

        writeln!(f, "}}")
    }
}
