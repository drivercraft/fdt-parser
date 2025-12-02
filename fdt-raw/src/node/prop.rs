use core::ffi::CStr;

use log::error;

use crate::{
    FdtError, Phandle, Status, Token,
    data::{Bytes, Reader},
};

/// 通用属性，包含名称和原始数据
#[derive(Clone)]
pub struct RawProperty<'a> {
    name: &'a str,
    data: &'a [u8],
}

impl<'a> RawProperty<'a> {
    pub fn new(name: &'a str, data: &'a [u8]) -> Self {
        Self { name, data }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 作为 u32 迭代器
    pub fn as_u32_iter(&self) -> U32Iter<'a> {
        U32Iter { data: self.data }
    }

    /// 作为字符串迭代器（用于 compatible 等属性）
    pub fn as_str_iter(&self) -> StrIter<'a> {
        StrIter { data: self.data }
    }

    /// 作为单个 u64 值
    pub fn as_u64(&self) -> Option<u64> {
        if self.data.len() != 8 {
            return None;
        }
        Some(u64::from_be_bytes(self.data.try_into().unwrap()))
    }

    /// 作为单个 u32 值
    pub fn as_u32(&self) -> Option<u32> {
        if self.data.len() != 4 {
            return None;
        }
        Some(u32::from_be_bytes(self.data.try_into().unwrap()))
    }

    /// 作为字符串
    pub fn as_str(&self) -> Option<&'a str> {
        if self.data.is_empty() {
            return None;
        }
        // 去除尾部的 null 终止符
        let data = if self.data.last() == Some(&0) {
            &self.data[..self.data.len() - 1]
        } else {
            self.data
        };
        core::str::from_utf8(data).ok()
    }
}

/// 类型化属性枚举
#[derive(Clone)]
pub enum Property<'a> {
    /// #address-cells 属性
    AddressCells(u8),
    /// #size-cells 属性
    SizeCells(u8),
    /// reg 属性（原始数据，需要根据 cells 解析）
    Reg(&'a [u8]),
    /// ranges 属性（原始数据，需要根据 cells 解析）
    Ranges(&'a [u8]),
    /// compatible 属性（字符串列表）
    Compatible(StrIter<'a>),
    /// model 属性
    Model(&'a str),
    /// status 属性
    Status(Status),
    /// phandle 属性
    Phandle(Phandle),
    /// linux,phandle 属性
    LinuxPhandle(Phandle),
    /// device_type 属性
    DeviceType(&'a str),
    /// interrupt-parent 属性
    InterruptParent(Phandle),
    /// interrupts 属性（原始数据）
    Interrupts(&'a [u8]),
    /// interrupt-cells 属性
    InterruptCells(u8),
    /// clocks 属性（原始数据）
    Clocks(&'a [u8]),
    /// clock-names 属性
    ClockNames(StrIter<'a>),
    /// dma-coherent 属性（无数据）
    DmaCoherent,
    /// 未识别的通用属性
    Unknown(RawProperty<'a>),
}

impl<'a> Property<'a> {
    /// 获取属性名称
    pub fn name(&self) -> &str {
        match self {
            Property::AddressCells(_) => "#address-cells",
            Property::SizeCells(_) => "#size-cells",
            Property::Reg(_) => "reg",
            Property::Ranges(_) => "ranges",
            Property::Compatible(_) => "compatible",
            Property::Model(_) => "model",
            Property::Status(_) => "status",
            Property::Phandle(_) => "phandle",
            Property::LinuxPhandle(_) => "linux,phandle",
            Property::DeviceType(_) => "device_type",
            Property::InterruptParent(_) => "interrupt-parent",
            Property::Interrupts(_) => "interrupts",
            Property::InterruptCells(_) => "#interrupt-cells",
            Property::Clocks(_) => "clocks",
            Property::ClockNames(_) => "clock-names",
            Property::DmaCoherent => "dma-coherent",
            Property::Unknown(raw) => raw.name(),
        }
    }

    /// 从名称和数据创建类型化属性
    fn from_raw(name: &'a str, data: &'a [u8]) -> Self {
        match name {
            "#address-cells" => {
                if data.len() == 4 {
                    let val = u32::from_be_bytes(data.try_into().unwrap()) as u8;
                    Property::AddressCells(val)
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "#size-cells" => {
                if data.len() == 4 {
                    let val = u32::from_be_bytes(data.try_into().unwrap()) as u8;
                    Property::SizeCells(val)
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "#interrupt-cells" => {
                if data.len() == 4 {
                    let val = u32::from_be_bytes(data.try_into().unwrap()) as u8;
                    Property::InterruptCells(val)
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "reg" => Property::Reg(data),
            "ranges" => Property::Ranges(data),
            "compatible" => Property::Compatible(StrIter { data }),
            "model" => {
                if let Some(s) = Self::parse_str(data) {
                    Property::Model(s)
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "status" => {
                if let Some(s) = Self::parse_str(data) {
                    match s {
                        "okay" | "ok" => Property::Status(Status::Okay),
                        "disabled" => Property::Status(Status::Disabled),
                        _ => Property::Unknown(RawProperty::new(name, data)),
                    }
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "phandle" => {
                if data.len() == 4 {
                    let val = u32::from_be_bytes(data.try_into().unwrap());
                    Property::Phandle(Phandle::from(val))
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "linux,phandle" => {
                if data.len() == 4 {
                    let val = u32::from_be_bytes(data.try_into().unwrap());
                    Property::LinuxPhandle(Phandle::from(val))
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "device_type" => {
                if let Some(s) = Self::parse_str(data) {
                    Property::DeviceType(s)
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "interrupt-parent" => {
                if data.len() == 4 {
                    let val = u32::from_be_bytes(data.try_into().unwrap());
                    Property::InterruptParent(Phandle::from(val))
                } else {
                    Property::Unknown(RawProperty::new(name, data))
                }
            }
            "interrupts" | "interrupts-extended" => Property::Interrupts(data),
            "clocks" => Property::Clocks(data),
            "clock-names" => Property::ClockNames(StrIter { data }),
            "dma-coherent" => Property::DmaCoherent,
            _ => Property::Unknown(RawProperty::new(name, data)),
        }
    }

    /// 解析字符串（去除 null 终止符）
    fn parse_str(data: &[u8]) -> Option<&str> {
        if data.is_empty() {
            return None;
        }
        let data = if data.last() == Some(&0) {
            &data[..data.len() - 1]
        } else {
            data
        };
        core::str::from_utf8(data).ok()
    }

    /// 尝试获取为通用属性
    pub fn as_raw(&self) -> Option<&RawProperty<'a>> {
        match self {
            Property::Unknown(raw) => Some(raw),
            _ => None,
        }
    }
}

/// 属性迭代器
pub struct PropIter<'a> {
    reader: Reader<'a>,
    strings: Bytes<'a>,
    finished: bool,
}

impl<'a> PropIter<'a> {
    pub(crate) fn new(reader: Reader<'a>, strings: Bytes<'a>) -> Self {
        Self {
            reader,
            strings,
            finished: false,
        }
    }

    /// 处理错误：输出错误日志并终止迭代
    fn handle_error(&mut self, err: FdtError) {
        error!("Property parse error: {}", err);
        self.finished = true;
    }

    /// 从 strings block 读取属性名
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
                    // 读取属性长度
                    let len_bytes = match self.reader.read_bytes(4) {
                        Some(b) => b,
                        None => {
                            self.handle_error(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            });
                            return None;
                        }
                    };
                    let len = u32::from_be_bytes(len_bytes.try_into().unwrap()) as usize;

                    // 读取属性名偏移
                    let nameoff_bytes = match self.reader.read_bytes(4) {
                        Some(b) => b,
                        None => {
                            self.handle_error(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            });
                            return None;
                        }
                    };
                    let nameoff = u32::from_be_bytes(nameoff_bytes.try_into().unwrap());

                    // 读取属性数据
                    let prop_data = if len > 0 {
                        match self.reader.read_bytes(len) {
                            Some(b) => b,
                            None => {
                                self.handle_error(FdtError::BufferTooSmall {
                                    pos: self.reader.position(),
                                });
                                return None;
                            }
                        }
                    } else {
                        &[]
                    };

                    // 读取属性名
                    let name = match self.read_prop_name(nameoff) {
                        Ok(n) => n,
                        Err(e) => {
                            self.handle_error(e);
                            return None;
                        }
                    };

                    // 对齐到 4 字节边界
                    self.align4();

                    return Some(Property::from_raw(name, prop_data));
                }
                Token::BeginNode | Token::EndNode | Token::End => {
                    // 遇到节点边界，回溯并终止属性迭代
                    self.reader.backtrack(4);
                    self.finished = true;
                    return None;
                }
                Token::Nop => {
                    // 忽略 NOP，继续
                    continue;
                }
                Token::Data(_) => {
                    // 非法 token
                    self.handle_error(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    });
                    return None;
                }
            }
        }
    }
}

/// u32 值迭代器
#[derive(Clone)]
pub struct U32Iter<'a> {
    data: &'a [u8],
}

impl Iterator for U32Iter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() < 4 {
            return None;
        }
        let value = u32::from_be_bytes(self.data[..4].try_into().unwrap());
        self.data = &self.data[4..];
        Some(value)
    }
}

/// 字符串迭代器（用于 compatible 等多字符串属性）
#[derive(Clone)]
pub struct StrIter<'a> {
    data: &'a [u8],
}

impl<'a> Iterator for StrIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }

        // 查找 null 终止符
        let end = self
            .data
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.data.len());

        if end == 0 {
            // 空字符串，跳过 null
            self.data = &self.data[1..];
            return self.next();
        }

        let s = core::str::from_utf8(&self.data[..end]).ok()?;

        // 跳过字符串和 null 终止符
        if end < self.data.len() {
            self.data = &self.data[end + 1..];
        } else {
            self.data = &[];
        }

        Some(s)
    }
}
