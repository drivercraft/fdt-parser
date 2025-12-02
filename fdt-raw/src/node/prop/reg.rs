//! Reg 属性相关类型

use super::super::RangeEntry;

/// Reg 条目信息
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegInfo {
    /// 父总线地址（经过 ranges 转换后的地址）
    pub address: u64,
    /// 子总线地址（原始地址）
    pub child_bus_address: u64,
    /// 区域大小
    pub size: Option<u64>,
}

impl RegInfo {
    /// 创建新的 RegInfo
    pub fn new(child_bus_address: u64, address: u64, size: Option<u64>) -> Self {
        Self {
            address,
            child_bus_address,
            size,
        }
    }
}

/// Reg 属性类型
#[derive(Clone)]
pub struct Reg<'a> {
    data: &'a [u8],
    address_cells: u8,
    size_cells: u8,
    ranges: heapless::Vec<RangeEntry, 16>,
}

impl<'a> Reg<'a> {
    /// 创建新的 Reg
    pub fn new(
        data: &'a [u8],
        address_cells: u8,
        size_cells: u8,
        ranges: heapless::Vec<RangeEntry, 16>,
    ) -> Self {
        Self {
            data,
            address_cells,
            size_cells,
            ranges,
        }
    }

    /// 获取原始数据
    pub fn as_slice(&self) -> &'a [u8] {
        self.data
    }

    /// 获取原始 u32 迭代器
    pub fn as_u32_iter(&self) -> super::U32Iter<'a> {
        super::U32Iter::new(self.data)
    }

    /// 获取 RegInfo 迭代器
    pub fn iter(&self) -> RegIter<'a> {
        RegIter {
            data: self.data,
            address_cells: self.address_cells,
            size_cells: self.size_cells,
            ranges: self.ranges.clone(),
        }
    }

    /// 获取所有 RegInfo 条目到数组
    pub fn to_array<const N: usize>(&self) -> heapless::Vec<RegInfo, N> {
        let mut result = heapless::Vec::new();
        for info in self.iter() {
            if result.push(info).is_err() {
                break;
            }
        }
        result
    }
}

/// Reg 迭代器
#[derive(Clone)]
pub struct RegIter<'a> {
    data: &'a [u8],
    address_cells: u8,
    size_cells: u8,
    ranges: heapless::Vec<RangeEntry, 16>,
}

impl RegIter<'_> {
    /// 根据 cells 数量读取值
    fn read_value(data: &[u8], cells: u8) -> Option<(u64, usize)> {
        let bytes_needed = (cells as usize) * 4;
        if data.len() < bytes_needed {
            return None;
        }
        let value = match cells {
            0 => 0,
            1 => u32::from_be_bytes(data[0..4].try_into().unwrap()) as u64,
            2 => u64::from_be_bytes(data[0..8].try_into().unwrap()),
            _ => {
                // 超过 2 cells，取低 64 位
                let offset = bytes_needed - 8;
                u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap())
            }
        };
        Some((value, bytes_needed))
    }

    /// 将子地址通过 ranges 转换为父地址
    fn translate_address(&self, child_addr: u64) -> u64 {
        let mut addr = child_addr;
        for range in self.ranges.iter().rev() {
            if let Some(translated) = range.translate(addr) {
                addr = translated;
            }
        }
        addr
    }
}

impl Iterator for RegIter<'_> {
    type Item = RegInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }

        // 读取地址
        let (child_bus_address, addr_bytes) = Self::read_value(self.data, self.address_cells)?;
        self.data = &self.data[addr_bytes..];

        // 读取大小
        let size = if self.size_cells > 0 {
            let (size_val, size_bytes) = Self::read_value(self.data, self.size_cells)?;
            self.data = &self.data[size_bytes..];
            Some(size_val)
        } else {
            None
        };

        // 转换地址
        let address = self.translate_address(child_bus_address);

        Some(RegInfo::new(child_bus_address, address, size))
    }
}
