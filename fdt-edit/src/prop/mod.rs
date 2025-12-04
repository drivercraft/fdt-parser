use alloc::{
    string::{String, ToString},
    vec::Vec,
};

// Re-export from fdt_raw
pub use fdt_raw::{Phandle, RegInfo, Status};

use crate::{Node, NodeOp};

mod cells;
mod phandle;

pub use cells::*;
pub use phandle::*;

#[enum_dispatch::enum_dispatch(Property)]
pub trait PropertyTrait {
    fn as_raw(&self) -> &RawProperty;
    fn as_raw_mut(&mut self) -> &mut RawProperty;
}

pub trait PropertyOp: PropertyTrait {
    /// 获取属性名称
    fn name(&self) -> &str {
        &self.as_raw().name
    }

    /// 获取属性数据
    fn data(&self) -> &[u8] {
        &self.as_raw().data
    }

    /// 获取可变属性数据
    fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.as_raw_mut().data
    }

    /// 属性数据是否为空
    fn is_empty(&self) -> bool {
        self.as_raw().data.is_empty()
    }

    /// 属性数据长度
    fn len(&self) -> usize {
        self.data().len()
    }

    fn as_str(&self) -> Option<&str> {
        let data = self.data();
        if data.is_empty() {
            return None;
        }
        let len = data.iter().position(|&b| b == 0).unwrap_or(data.len());

        core::str::from_utf8(&data[..len]).ok()
    }

    fn as_u32_vec(&self) -> Vec<u32> {
        if self.data().len() % 4 != 0 {
            return vec![];
        }
        let mut result = Vec::new();
        for chunk in self.data().chunks(4) {
            let value = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            result.push(value);
        }
        result
    }

    fn as_u64_vec(&self) -> Vec<u64> {
        if self.len() % 8 != 0 {
            return vec![];
        }
        let mut result = Vec::new();
        for chunk in self.data().chunks(8) {
            let value = u64::from_be_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
            result.push(value);
        }
        result
    }

    fn as_string_list(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut start = 0;
        for (i, &byte) in self.data().iter().enumerate() {
            if byte == 0 {
                if i == start {
                    // 连续的 null 字节，跳过
                    start += 1;
                    continue;
                }
                let s = core::str::from_utf8(&self.data()[start..i]).ok().unwrap();
                result.push(s.to_string());
                start = i + 1;
            }
        }
        // 处理最后一个字符串（如果没有以 null 结尾）
        if start < self.len() {
            let s = core::str::from_utf8(&self.data()[start..]).ok().unwrap();
            result.push(s.to_string());
        }
        result
    }
}

/// Ranges 条目信息
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangesEntry {
    /// 子总线地址
    pub child_bus_address: u64,
    /// 父总线地址
    pub parent_bus_address: u64,
    /// 区域长度
    pub length: u64,
}

impl RangesEntry {
    /// 创建新的 RangesEntry
    pub fn new(child_bus_address: u64, parent_bus_address: u64, length: u64) -> Self {
        Self {
            child_bus_address,
            parent_bus_address,
            length,
        }
    }
}

/// 原始属性（未识别的通用属性）
#[derive(Clone, Debug)]
pub struct RawProperty {
    pub(crate) name: String,
    pub(crate) data: Vec<u8>,
}

impl RawProperty {
    /// 创建新的原始属性
    pub fn new(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            data,
        }
    }

    /// 创建空属性
    pub fn empty(name: impl Into<String>) -> Self {
        Self::new(name, Vec::new())
    }

    /// 创建 u32 属性
    pub fn from_u32(name: impl Into<String>, value: u32) -> Self {
        Self::new(name, value.to_be_bytes().to_vec())
    }

    /// 创建 u64 属性
    pub fn from_u64(name: impl Into<String>, value: u64) -> Self {
        Self::new(name, value.to_be_bytes().to_vec())
    }

    /// 创建字符串属性
    pub fn from_string(name: impl Into<String>, value: &str) -> Self {
        let mut data = value.as_bytes().to_vec();
        data.push(0);
        Self::new(name, data)
    }

    /// 创建字符串列表属性
    pub fn from_string_list(name: impl Into<String>, values: &[&str]) -> Self {
        let mut data = Vec::new();
        for s in values {
            data.extend_from_slice(s.as_bytes());
            data.push(0);
        }
        Self::new(name, data)
    }
}

impl PropertyTrait for RawProperty {
    fn as_raw(&self) -> &RawProperty {
        self
    }

    fn as_raw_mut(&mut self) -> &mut RawProperty {
        self
    }
}

impl PropertyOp for RawProperty {}

#[enum_dispatch::enum_dispatch]
/// 可编辑的属性（类型化枚举）
#[derive(Clone, Debug)]
pub enum Property {
    /// #address-cells 属性
    U32(U32),

    StringList(StringList),
    // /// reg 属性（已解析）
    // Reg(Vec<RegInfo>),
    // /// ranges 属性（空表示 1:1 映射）
    // Ranges {
    //     entries: Vec<RangesEntry>,
    //     child_address_cells: u8,
    //     parent_address_cells: u8,
    //     size_cells: u8,
    // },
    // /// compatible 属性（字符串列表）
    // Compatible(Vec<String>),
    // /// model 属性
    // Model(String),
    /// status 属性
    Status(PropStatus),
    /// phandle 属性
    Phandle(PropPhandle),

    Str(FStr),
    // /// linux,phandle 属性
    // LinuxPhandle(Phandle),
    // /// device_type 属性
    // DeviceType(String),
    // /// interrupt-parent 属性
    // InterruptParent(Phandle),
    // /// clock-names 属性
    // ClockNames(Vec<String>),
    // /// dma-coherent 属性（无数据）
    // DmaCoherent,
    /// 原始属性（未识别的通用属性）
    Raw(RawProperty),
}

impl Property {
    // /// 获取属性名称
    // pub fn name(&self) -> &str {
    //     match self {
    //         Property::AddressCells(_) => "#address-cells",
    //         Property::SizeCells(_) => "#size-cells",
    //         Property::InterruptCells(_) => "#interrupt-cells",
    //         Property::Reg { .. } => "reg",
    //         Property::Ranges { .. } => "ranges",
    //         Property::Compatible(_) => "compatible",
    //         Property::Model(_) => "model",
    //         Property::Status(_) => "status",
    //         Property::Phandle(_) => "phandle",
    //         Property::LinuxPhandle(_) => "linux,phandle",
    //         Property::DeviceType(_) => "device_type",
    //         Property::InterruptParent(_) => "interrupt-parent",
    //         Property::ClockNames(_) => "clock-names",
    //         Property::DmaCoherent => "dma-coherent",
    //         Property::Raw(raw) => raw.name(),
    //     }
    // }

    // /// 将属性序列化为二进制数据
    // pub fn to_bytes(&self, node: &Node) -> Vec<u8> {
    //     let address_cells = node.address_cells().unwrap_or(2);
    //     let size_cells = node.size_cells().unwrap_or(1);
    //     self.to_bytes_with_cells(node, address_cells, size_cells)
    // }

    /// 将属性序列化为二进制数据，使用指定的父节点 address_cells 和 size_cells
    pub fn to_bytes_with_cells(
        &self,
        _node: &Node,
        parent_address_cells: u8,
        parent_size_cells: u8,
    ) -> Vec<u8> {
        todo!()
        //     match self {
        //         Property::AddressCells(v) => (*v as u32).to_be_bytes().to_vec(),
        //         Property::SizeCells(v) => (*v as u32).to_be_bytes().to_vec(),
        //         Property::InterruptCells(v) => (*v as u32).to_be_bytes().to_vec(),
        //         Property::Reg(entries) => {
        //             let mut data = Vec::new();
        //             for entry in entries {
        //                 write_cells(&mut data, entry.address, parent_address_cells);
        //                 if let Some(size) = entry.size {
        //                     write_cells(&mut data, size, parent_size_cells);
        //                 }
        //             }
        //             data
        //         }
        //         Property::Ranges {
        //             entries,
        //             child_address_cells,
        //             parent_address_cells,
        //             size_cells,
        //         } => {
        //             let mut data = Vec::new();
        //             for entry in entries {
        //                 write_cells(&mut data, entry.child_bus_address, *child_address_cells);
        //                 write_cells(&mut data, entry.parent_bus_address, *parent_address_cells);
        //                 write_cells(&mut data, entry.length, *size_cells);
        //             }
        //             data
        //         }
        //         Property::Compatible(strs) => {
        //             let mut data = Vec::new();
        //             for s in strs {
        //                 data.extend_from_slice(s.as_bytes());
        //                 data.push(0);
        //             }
        //             data
        //         }
        //         Property::Model(s) => {
        //             let mut data = s.as_bytes().to_vec();
        //             data.push(0);
        //             data
        //         }
        //         Property::Status(status) => {
        //             let s = match status {
        //                 Status::Okay => "okay",
        //                 Status::Disabled => "disabled",
        //             };
        //             let mut data = s.as_bytes().to_vec();
        //             data.push(0);
        //             data
        //         }
        //         Property::Phandle(v) => (v.as_usize() as u32).to_be_bytes().to_vec(),
        //         Property::LinuxPhandle(v) => (v.as_usize() as u32).to_be_bytes().to_vec(),
        //         Property::DeviceType(s) => {
        //             let mut data = s.as_bytes().to_vec();
        //             data.push(0);
        //             data
        //         }
        //         Property::InterruptParent(v) => (v.as_usize() as u32).to_be_bytes().to_vec(),
        //         Property::ClockNames(strs) => {
        //             let mut data = Vec::new();
        //             for s in strs {
        //                 data.extend_from_slice(s.as_bytes());
        //                 data.push(0);
        //             }
        //             data
        //         }
        //         Property::DmaCoherent => Vec::new(),
        //         Property::Raw(raw) => raw.data().to_vec(),
        //     }
    }

    // /// 属性数据是否为空
    // pub fn is_empty(&self) -> bool {
    //     match self {
    //         Property::DmaCoherent => true,
    //         Property::Ranges { entries, .. } => entries.is_empty(),
    //         Property::Raw(raw) => raw.is_empty(),
    //         _ => false,
    //     }
    // }

    // ========== 构造器方法 ==========

    // /// 创建 ranges 属性
    // pub fn ranges(
    //     entries: Vec<RangesEntry>,
    //     child_address_cells: u8,
    //     parent_address_cells: u8,
    //     size_cells: u8,
    // ) -> Self {
    //     Property::Ranges {
    //         entries,
    //         child_address_cells,
    //         parent_address_cells,
    //         size_cells,
    //     }
    // }

    // /// 创建 compatible 属性
    // pub fn compatible(values: Vec<String>) -> Self {
    //     Property::Compatible(values)
    // }

    // /// 从字符串切片创建 compatible 属性
    // pub fn compatible_from_strs(values: &[&str]) -> Self {
    //     Property::Compatible(values.iter().map(|s| String::from(*s)).collect())
    // }

    // /// 创建 model 属性
    // pub fn model(value: impl Into<String>) -> Self {
    //     Property::Model(value.into())
    // }

    // /// 创建 status 属性
    // pub fn status(status: Status) -> Self {
    //     Property::Status(status)
    // }

    // /// 创建 status = "okay" 属性
    // pub fn status_okay() -> Self {
    //     Property::Status(Status::Okay)
    // }

    // /// 创建 status = "disabled" 属性
    // pub fn status_disabled() -> Self {
    //     Property::Status(Status::Disabled)
    // }

    // /// 创建 phandle 属性
    // pub fn phandle(value: u32) -> Self {
    //     Property::Phandle(Phandle::from(value))
    // }

    // /// 创建 linux,phandle 属性
    // pub fn linux_phandle(value: u32) -> Self {
    //     Property::LinuxPhandle(Phandle::from(value))
    // }

    // /// 创建 device_type 属性
    // pub fn device_type(value: impl Into<String>) -> Self {
    //     Property::DeviceType(value.into())
    // }

    // /// 创建 interrupt-parent 属性
    // pub fn interrupt_parent(phandle: u32) -> Self {
    //     Property::InterruptParent(Phandle::from(phandle))
    // }

    // /// 创建 clock-names 属性
    // pub fn clock_names(values: Vec<String>) -> Self {
    //     Property::ClockNames(values)
    // }

    // /// 创建 dma-coherent 属性
    // pub fn dma_coherent() -> Self {
    //     Property::DmaCoherent
    // }

    // /// 创建原始属性（通用属性）
    // pub fn raw(name: impl Into<String>, data: Vec<u8>) -> Self {
    //     Property::Raw(RawProperty::new(name, data))
    // }

    // /// 创建 u32 原始属性
    // pub fn raw_u32(name: impl Into<String>, value: u32) -> Self {
    //     Property::Raw(RawProperty::from_u32(name, value))
    // }

    // /// 创建 u64 原始属性
    // pub fn raw_u64(name: impl Into<String>, value: u64) -> Self {
    //     Property::Raw(RawProperty::from_u64(name, value))
    // }

    // /// 创建字符串原始属性
    // pub fn raw_string(name: impl Into<String>, value: &str) -> Self {
    //     Property::Raw(RawProperty::from_string(name, value))
    // }

    // /// 创建字符串列表原始属性
    // pub fn raw_string_list(name: impl Into<String>, values: &[&str]) -> Self {
    //     Property::Raw(RawProperty::from_string_list(name, values))
    // }

    // /// 创建空原始属性
    // pub fn raw_empty(name: impl Into<String>) -> Self {
    //     Property::Raw(RawProperty::empty(name))
    // }
}

/// 根据 cells 数量写入值
fn write_cells(data: &mut Vec<u8>, value: u64, cells: u8) {
    match cells {
        0 => {}
        1 => data.extend_from_slice(&(value as u32).to_be_bytes()),
        2 => data.extend_from_slice(&value.to_be_bytes()),
        _ => {
            // 超过 2 cells，先填充 0，再写入 64 位值
            for _ in 0..(cells as usize - 2) {
                data.extend_from_slice(&0u32.to_be_bytes());
            }
            data.extend_from_slice(&value.to_be_bytes());
        }
    }
}

impl<'a> From<fdt_raw::Property<'a>> for Property {
    fn from(prop: fdt_raw::Property<'a>) -> Self {
        let name = prop.name().to_string();
        match prop {
            fdt_raw::Property::AddressCells(v) => Property::U32(U32::new(prop.name(), v as _)),
            fdt_raw::Property::SizeCells(v) => Property::U32(U32::new(prop.name(), v as _)),
            fdt_raw::Property::Reg(reg) => {
                let data = reg.as_slice().to_vec();
                Property::Raw(RawProperty::new(&name, data))
            }
            fdt_raw::Property::Compatible(str_iter) => {
                Property::StringList(StringList::new(&name, str_iter))
            }
            fdt_raw::Property::Status(status) => Property::Status(PropStatus::new(status)),
            fdt_raw::Property::Phandle(phandle) => {
                Property::Phandle(PropPhandle::new(prop.name(), phandle))
            }
            fdt_raw::Property::DeviceType(v) => Property::Str(FStr::new(prop.name(), v)),
            fdt_raw::Property::InterruptParent(phandle) => {
                Property::Phandle(PropPhandle::new(prop.name(), phandle))
            }
            fdt_raw::Property::InterruptCells(v) => Property::U32(U32::new(prop.name(), v as _)),
            fdt_raw::Property::ClockNames(str_iter) => {
                Property::StringList(StringList::new(&name, str_iter))
            }
            fdt_raw::Property::DmaCoherent => todo!(),
            fdt_raw::Property::Unknown(raw_property) => {
                let data = raw_property.data().to_vec();
                Property::Raw(RawProperty::new(&name, data))
            }
        }
    }
}

impl PropertyOp for Property {}
