use core::fmt;

use crate::{
    Chosen, FdtError, Memory, MemoryReservation, Node, data::Bytes, header::Header, iter::FdtIter,
};

/// Memory reservation block iterator
pub struct MemoryReservationIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for MemoryReservationIter<'a> {
    type Item = MemoryReservation;

    fn next(&mut self) -> Option<Self::Item> {
        // 确保我们有足够的数据来读取地址和大小（各8字节）
        if self.offset + 16 > self.data.len() {
            return None;
        }

        // 读取地址（8字节，大端序）
        let address_bytes = &self.data[self.offset..self.offset + 8];
        let address = u64::from_be_bytes(address_bytes.try_into().unwrap());
        self.offset += 8;

        // 读取大小（8字节，大端序）
        let size_bytes = &self.data[self.offset..self.offset + 8];
        let size = u64::from_be_bytes(size_bytes.try_into().unwrap());
        self.offset += 8;

        // 检查是否到达终止符（地址和大小都为0）
        if address == 0 && size == 0 {
            return None;
        }

        Some(MemoryReservation { address, size })
    }
}

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

        let data_slice = unsafe { core::slice::from_raw_parts(ptr, header.totalsize as _) };
        let data = Bytes::new(data_slice);

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

    /// Get an iterator over memory reservation entries
    pub fn memory_reservations(&self) -> MemoryReservationIter<'a> {
        MemoryReservationIter {
            data: self.data.as_slice(),
            offset: self.header.off_mem_rsvmap as usize,
        }
    }

    pub fn chosen(&self) -> Option<Chosen<'a>> {
        for node in self.all_nodes() {
            if let Node::Chosen(c) = node {
                return Some(c);
            }
        }
        None
    }

    pub fn memory(&self) -> Option<Memory<'a>> {
        for node in self.all_nodes() {
            if let Node::Memory(m) = node {
                return Some(m);
            }
        }
        None
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
                if let Some(v) = prop.as_address_cells() {
                    writeln!(f, "#address-cells: {}", v)?;
                } else if let Some(v) = prop.as_size_cells() {
                    writeln!(f, "#size-cells: {}", v)?;
                } else if let Some(v) = prop.as_interrupt_cells() {
                    writeln!(f, "#interrupt-cells: {}", v)?;
                } else if let Some(s) = prop.as_status() {
                    writeln!(f, "status: {:?}", s)?;
                } else if let Some(p) = prop.as_phandle() {
                    writeln!(f, "phandle: {}", p)?;
                } else {
                    // 默认处理未知属性
                    if prop.is_empty() {
                        writeln!(f, "{}", prop.name())?;
                    } else if let Some(s) = prop.as_str() {
                        writeln!(f, "{}: \"{}\"", prop.name(), s)?;
                    } else if prop.len() == 4 {
                        let v = u32::from_be_bytes(prop.data().as_slice().try_into().unwrap());
                        writeln!(f, "{}: {:#x}", prop.name(), v)?;
                    } else {
                        writeln!(f, "{}: <{} bytes>", prop.name(), prop.len())?;
                    }
                }
            }
        }

        writeln!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;

    #[test]
    fn test_memory_reservation_iterator() {
        // 创建一个简单的测试数据：一个内存保留条目 + 终止符
        let mut test_data = [0u8; 32];

        // 地址: 0x80000000, 大小: 0x10000000 (256MB)
        test_data[0..8].copy_from_slice(&0x80000000u64.to_be_bytes());
        test_data[8..16].copy_from_slice(&0x10000000u64.to_be_bytes());
        // 终止符: address=0, size=0
        test_data[16..24].copy_from_slice(&0u64.to_be_bytes());
        test_data[24..32].copy_from_slice(&0u64.to_be_bytes());

        let iter = MemoryReservationIter {
            data: &test_data,
            offset: 0,
        };

        let reservations: Vec<MemoryReservation, 4> = iter.collect();
        assert_eq!(reservations.len(), 1);
        assert_eq!(reservations[0].address, 0x80000000);
        assert_eq!(reservations[0].size, 0x10000000);
    }

    #[test]
    fn test_empty_memory_reservation_iterator() {
        // 只有终止符
        let mut test_data = [0u8; 16];
        test_data[0..8].copy_from_slice(&0u64.to_be_bytes());
        test_data[8..16].copy_from_slice(&0u64.to_be_bytes());

        let iter = MemoryReservationIter {
            data: &test_data,
            offset: 0,
        };

        let reservations: Vec<MemoryReservation, 4> = iter.collect();
        assert_eq!(reservations.len(), 0);
    }
}
