use alloc::{
    string::{String, ToString},
    vec::Vec,
};

// Re-export from fdt_raw
pub use fdt_raw::{Phandle, RegInfo, Status};

use crate::FdtContext;

#[derive(Clone, Debug)]
pub struct Property {
    pub name: String,
    pub kind: PropertyKind,
}

#[derive(Clone, Debug)]
pub enum PropertyKind {
    Num(u64),
    NumVec(Vec<u64>),
    Str(String),
    StringList(Vec<String>),
    Status(Status),
    Phandle(Phandle),
    Bool,
    Reg(Vec<Reg>),
    Raw(RawProperty),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reg {
    /// cpu side address
    pub address: u64,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegFixed {
    /// cpu side address
    pub address: u64,
    pub child_bus_address: u64,
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RawProperty(pub Vec<u8>);

impl RawProperty {
    pub fn data(&self) -> &[u8] {
        &self.0
    }

    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_u32_vec(&self) -> Vec<u32> {
        if !self.0.len().is_multiple_of(4) {
            return vec![];
        }
        let mut result = Vec::new();
        for chunk in self.0.chunks(4) {
            let value = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            result.push(value);
        }
        result
    }

    pub fn as_u64_vec(&self) -> Vec<u64> {
        if !self.0.len().is_multiple_of(8) {
            return vec![];
        }
        let mut result = Vec::new();
        for chunk in self.0.chunks(8) {
            let value = u64::from_be_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
            result.push(value);
        }
        result
    }

    pub fn as_string_list(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut start = 0;
        for (i, &byte) in self.0.iter().enumerate() {
            if byte == 0 {
                if i == start {
                    // 连续的 null 字节，跳过
                    start += 1;
                    continue;
                }
                let s = core::str::from_utf8(&self.0[start..i]).ok().unwrap();
                result.push(s.to_string());
                start = i + 1;
            }
        }
        // 处理最后一个字符串（如果没有以 null 结尾）
        if start < self.0.len() {
            let s = core::str::from_utf8(&self.0[start..]).ok().unwrap();
            result.push(s.to_string());
        }
        result
    }

    pub fn as_str(&self) -> Option<&str> {
        if self.0.is_empty() {
            return None;
        }
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(self.0.len());

        core::str::from_utf8(&self.0[..len]).ok()
    }

    pub fn set_str_list(&mut self, strings: &[&str]) {
        self.0.clear();
        for s in strings {
            self.0.extend_from_slice(s.as_bytes());
            self.0.push(0);
        }
    }

    pub fn set_u32_vec(&mut self, values: &[u32]) {
        self.0.clear();
        for &v in values {
            self.0.extend_from_slice(&v.to_be_bytes());
        }
    }

    pub fn set_u64(&mut self, value: u64) {
        self.0.clear();
        self.0.extend_from_slice(&value.to_be_bytes());
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

impl Property {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 将属性序列化为二进制数据
    pub fn encode(&self, ctx: &FdtContext) -> Vec<u8> {
        match &self.kind {
            PropertyKind::Num(v) => (*v as u32).to_be_bytes().to_vec(),
            PropertyKind::NumVec(values) => {
                let mut data = Vec::new();
                for v in values {
                    data.extend_from_slice(&(*v as u32).to_be_bytes());
                }
                data
            }
            PropertyKind::Str(s) => {
                let mut data = s.as_bytes().to_vec();
                data.push(0);
                data
            }
            PropertyKind::StringList(strs) => {
                let mut data = Vec::new();
                for s in strs {
                    data.extend_from_slice(s.as_bytes());
                    data.push(0);
                }
                data
            }
            PropertyKind::Status(status) => {
                let s = match status {
                    Status::Okay => "okay",
                    Status::Disabled => "disabled",
                };
                let mut data = s.as_bytes().to_vec();
                data.push(0);
                data
            }
            PropertyKind::Phandle(v) => (v.as_usize() as u32).to_be_bytes().to_vec(),
            PropertyKind::Bool => Vec::new(),
            PropertyKind::Reg(entries) => {
                let mut data = Vec::new();
                for entry in entries {
                    write_cells(&mut data, entry.address, ctx.parent_address_cells());
                    if let Some(size) = entry.size {
                        write_cells(&mut data, size, ctx.parent_size_cells());
                    }
                }
                data
            }
            PropertyKind::Raw(raw) => raw.data().to_vec(),
        }
    }
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
            fdt_raw::Property::AddressCells(v) => Property {
                name,
                kind: PropertyKind::Num(v as _),
            },
            fdt_raw::Property::SizeCells(v) => Property {
                name,
                kind: PropertyKind::Num(v as _),
            },
            fdt_raw::Property::Reg(reg) => {
                let entries = reg
                    .iter()
                    .map(|e| Reg {
                        address: e.address,
                        size: e.size,
                    })
                    .collect();
                Property {
                    name,
                    kind: PropertyKind::Reg(entries), // Placeholder
                }
            }
            fdt_raw::Property::Compatible(str_iter) => {
                let values = str_iter.map(|s| s.to_string()).collect();
                Property {
                    name,
                    kind: PropertyKind::StringList(values),
                }
            }
            fdt_raw::Property::Status(status) => Property {
                name,
                kind: PropertyKind::Status(status),
            },
            fdt_raw::Property::Phandle(phandle) => Property {
                name,
                kind: PropertyKind::Phandle(phandle),
            },
            fdt_raw::Property::DeviceType(v) => Property {
                name,
                kind: PropertyKind::Str(v.to_string()),
            },
            fdt_raw::Property::InterruptParent(phandle) => Property {
                name,
                kind: PropertyKind::Phandle(phandle),
            },
            fdt_raw::Property::InterruptCells(v) => Property {
                name,
                kind: PropertyKind::Num(v as _),
            },
            fdt_raw::Property::ClockNames(str_iter) => {
                let values = str_iter.map(|s| s.to_string()).collect();
                Property {
                    name,
                    kind: PropertyKind::StringList(values),
                }
            }
            fdt_raw::Property::DmaCoherent => Property {
                name,
                kind: PropertyKind::Bool,
            },
            fdt_raw::Property::Unknown(raw_property) => {
                if raw_property.name().ends_with("-cells") && raw_property.name().starts_with("#") {
                    let data = raw_property.data();
                    if data.len() == 4 {
                        let value = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                        return Property {
                            name,
                            kind: PropertyKind::Num(value as _),
                        };
                    }
                }

                match name.as_str() {
                    "clock-output-names" => {
                        let values = raw_property.as_str_iter().map(|s| s.to_string()).collect();
                        Property {
                            name,
                            kind: PropertyKind::StringList(values),
                        }
                    }
                    "clock-frequency" | "clock-accuracy" => {
                        let val = raw_property.as_u32().unwrap();
                        Property {
                            name,
                            kind: PropertyKind::Num(val as _),
                        }
                    }
                    _ => Property {
                        name,
                        kind: PropertyKind::Raw(RawProperty(raw_property.data().to_vec())),
                    },
                }
            }
        }
    }
}
