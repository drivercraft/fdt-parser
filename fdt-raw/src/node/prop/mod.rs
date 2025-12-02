//! 属性相关类型和迭代器

mod reg;

use core::ffi::CStr;
use core::fmt;

use log::error;

pub use reg::{Reg, RegInfo, RegIter};

use super::NodeContext;
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
        U32Iter::new(self.data)
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
    /// reg 属性（已解析）
    Reg(Reg<'a>),
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
    /// interrupt-cells 属性
    InterruptCells(u8),
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
            Property::Compatible(_) => "compatible",
            Property::Model(_) => "model",
            Property::Status(_) => "status",
            Property::Phandle(_) => "phandle",
            Property::LinuxPhandle(_) => "linux,phandle",
            Property::DeviceType(_) => "device_type",
            Property::InterruptParent(_) => "interrupt-parent",
            Property::InterruptCells(_) => "#interrupt-cells",
            Property::ClockNames(_) => "clock-names",
            Property::DmaCoherent => "dma-coherent",
            Property::Unknown(raw) => raw.name(),
        }
    }

    /// 从名称和数据创建类型化属性
    fn from_raw(name: &'a str, data: &'a [u8], context: &NodeContext) -> Self {
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
            "reg" => {
                // 使用 context 中的 cells 信息解析 reg
                let reg = Reg::new(
                    data,
                    context.parent_address_cells,
                    context.parent_size_cells,
                );
                Property::Reg(reg)
            }
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

impl fmt::Display for Property<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Property::AddressCells(v) => write!(f, "#address-cells = <{:#x}>", v),
            Property::SizeCells(v) => write!(f, "#size-cells = <{:#x}>", v),
            Property::InterruptCells(v) => write!(f, "#interrupt-cells = <{:#x}>", v),
            Property::Reg(reg) => {
                write!(f, "reg = ")?;
                format_bytes(f, reg.as_slice())
            }

            Property::Compatible(iter) => {
                write!(f, "compatible = ")?;
                let mut first = true;
                for s in iter.clone() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\"", s)?;
                    first = false;
                }
                Ok(())
            }
            Property::Model(s) => write!(f, "model = \"{}\"", s),
            Property::DeviceType(s) => write!(f, "device_type = \"{}\"", s),
            Property::Status(s) => write!(f, "status = \"{:?}\"", s),
            Property::Phandle(p) => write!(f, "phandle = {}", p),
            Property::LinuxPhandle(p) => write!(f, "linux,phandle = {}", p),
            Property::InterruptParent(p) => write!(f, "interrupt-parent = {}", p),
            Property::ClockNames(iter) => {
                write!(f, "clock-names = ")?;
                let mut first = true;
                for s in iter.clone() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\"", s)?;
                    first = false;
                }
                Ok(())
            }
            Property::DmaCoherent => write!(f, "dma-coherent"),
            Property::Unknown(raw) => {
                if raw.is_empty() {
                    write!(f, "{}", raw.name())
                } else if let Some(s) = raw.as_str() {
                    // 检查是否有多个字符串
                    if raw.data().iter().filter(|&&b| b == 0).count() > 1 {
                        write!(f, "{} = ", raw.name())?;
                        let mut first = true;
                        for s in raw.as_str_iter() {
                            if !first {
                                write!(f, ", ")?;
                            }
                            write!(f, "\"{}\"", s)?;
                            first = false;
                        }
                        Ok(())
                    } else {
                        write!(f, "{} = \"{}\"", raw.name(), s)
                    }
                } else if raw.len() == 4 {
                    // 单个 u32
                    let v = u32::from_be_bytes(raw.data().try_into().unwrap());
                    write!(f, "{} = <{:#x}>", raw.name(), v)
                } else {
                    // 原始字节
                    write!(f, "{} = ", raw.name())?;
                    format_bytes(f, raw.data())
                }
            }
        }
    }
}

/// 格式化字节数组为 DTS 格式
fn format_bytes(f: &mut fmt::Formatter<'_>, data: &[u8]) -> fmt::Result {
    if data.len().is_multiple_of(4) {
        // 按 u32 格式化
        write!(f, "<")?;
        let mut first = true;
        for chunk in data.chunks(4) {
            if !first {
                write!(f, " ")?;
            }
            let v = u32::from_be_bytes(chunk.try_into().unwrap());
            write!(f, "{:#x}", v)?;
            first = false;
        }
        write!(f, ">")
    } else {
        // 按字节格式化
        write!(f, "[")?;
        for (i, b) in data.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{:02x}", b)?;
        }
        write!(f, "]")
    }
}

/// 属性迭代器
pub struct PropIter<'a> {
    reader: Reader<'a>,
    strings: Bytes<'a>,
    context: NodeContext,
    finished: bool,
}

impl<'a> PropIter<'a> {
    pub(crate) fn new(reader: Reader<'a>, strings: Bytes<'a>, context: NodeContext) -> Self {
        Self {
            reader,
            strings,
            context,
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

                    return Some(Property::from_raw(name, prop_data, &self.context));
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

impl<'a> U32Iter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }
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
