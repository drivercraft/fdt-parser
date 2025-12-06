//! Reg 属性相关类型

use crate::data::Reader;

/// Reg 条目信息
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegInfo {
    /// 地址
    pub address: u64,
    /// 区域大小
    pub size: Option<u64>,
}

impl RegInfo {
    /// 创建新的 RegInfo
    pub fn new(address: u64, size: Option<u64>) -> Self {
        Self { address, size }
    }
}

/// Reg 迭代器
#[derive(Clone)]
pub struct RegIter<'a> {
    reader: Reader<'a>,
    address_cells: u8,
    size_cells: u8,
}

impl<'a> RegIter<'a> {
    pub(crate) fn new(reader: Reader<'a>, address_cells: u8, size_cells: u8) -> RegIter<'a> {
        RegIter {
            reader,
            address_cells,
            size_cells,
        }
    }
}

impl Iterator for RegIter<'_> {
    type Item = RegInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let address;
        let mut size: Option<u64> = None;
        if self.address_cells == 1 {
            address = self.reader.read_u32().map(|addr| addr as u64)?;
        } else if self.address_cells == 2 {
            address = self.reader.read_u64()?;
        } else {
            return None;
        }

        if self.size_cells == 1 {
            size = Some(self.reader.read_u32().map(|s| s as u64)?);
        } else if self.size_cells == 2 {
            size = Some(self.reader.read_u64()?);
        }

        Some(RegInfo::new(address, size))
    }
}
