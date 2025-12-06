//! 属性相关类型和迭代器

mod reg;

use core::ffi::CStr;
use core::fmt;

use log::error;

pub use reg::{Reg, RegInfo, RegIter};

use super::NodeContext;
use crate::{
    FdtError, Phandle, Status, Token,
    data::{Bytes, Reader, StrIter, U32Iter},
};

/// 通用属性，包含名称和原始数据
#[derive(Clone)]
pub struct Property<'a> {
    name: &'a str,
    data: Bytes<'a>,
}

impl<'a> Property<'a> {
    pub fn new(name: &'a str, data: Bytes<'a>) -> Self {
        Self { name, data }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn data(&self) -> Bytes<'a> {
        self.data.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 作为 u32 迭代器
    pub fn as_u32_iter(&self) -> U32Iter<'a> {
        self.data.as_u32_iter()
    }

    /// 作为字符串迭代器（用于 compatible 等属性）
    pub fn as_str_iter(&self) -> StrIter<'a> {
        self.data.as_str_iter()
    }

    /// 获取数据作为字节切片
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// 作为单个 u64 值
    pub fn as_u64(&self) -> Option<u64> {
        let mut iter = self.as_u32_iter();
        let high = iter.next()? as u64;
        let low = iter.next()? as u64;
        if iter.next().is_some() {
            return None;
        }
        Some((high << 32) | low)
    }

    /// 作为单个 u32 值
    pub fn as_u32(&self) -> Option<u32> {
        let mut iter = self.as_u32_iter();
        let value = iter.next()?;
        if iter.next().is_some() {
            return None;
        }
        Some(value)
    }

    /// 作为字符串
    pub fn as_str(&self) -> Option<&'a str> {
        let bytes = self.data.as_slice();
        let cstr = CStr::from_bytes_until_nul(bytes).ok()?;
        cstr.to_str().ok()
    }

    /// 获取为 #address-cells 值
    pub fn as_address_cells(&self) -> Option<u8> {
        if self.name == "#address-cells" {
            self.as_u32().map(|v| v as u8)
        } else {
            None
        }
    }

    /// 获取为 #size-cells 值
    pub fn as_size_cells(&self) -> Option<u8> {
        if self.name == "#size-cells" {
            self.as_u32().map(|v| v as u8)
        } else {
            None
        }
    }

    /// 获取为 #interrupt-cells 值
    pub fn as_interrupt_cells(&self) -> Option<u8> {
        if self.name == "#interrupt-cells" {
            self.as_u32().map(|v| v as u8)
        } else {
            None
        }
    }

    /// 获取为 status 枚举
    pub fn as_status(&self) -> Option<Status> {
        let v = self.as_str()?;
        if self.name == "status" {
            match v {
                "okay" | "ok" => Some(Status::Okay),
                "disabled" => Some(Status::Disabled),
                _ => None,
            }
        } else {
            None
        }
    }

    /// 获取为 phandle
    pub fn as_phandle(&self) -> Option<Phandle> {
        if self.name == "phandle" {
            self.as_u32().map(Phandle::from)
        } else {
            None
        }
    }

    /// 获取为 device_type 字符串
    pub fn as_device_type(&self) -> Option<&'a str> {
        if self.name == "device_type" {
            self.as_str()
        } else {
            None
        }
    }

    /// 获取为 interrupt-parent
    pub fn as_interrupt_parent(&self) -> Option<Phandle> {
        if self.name == "interrupt-parent" {
            self.as_u32().map(Phandle::from)
        } else {
            None
        }
    }

    /// 获取为 clock-names 字符串列表
    pub fn as_clock_names(&self) -> Option<StrIter<'a>> {
        if self.name == "clock-names" {
            Some(self.as_str_iter())
        } else {
            None
        }
    }

    /// 获取为 compatible 字符串列表
    pub fn as_compatible(&self) -> Option<StrIter<'a>> {
        if self.name == "compatible" {
            Some(self.as_str_iter())
        } else {
            None
        }
    }

    /// 获取为 reg 属性（需要 context 信息）
    pub fn as_reg(&self, address_cells: u32, size_cells: u32) -> Option<Reg<'a>> {
        if self.name == "reg" {
            Some(Reg::new(self.data.as_slice(), address_cells as u8, size_cells as u8))
        } else {
            None
        }
    }

    /// 是否为 dma-coherent 属性
    pub fn is_dma_coherent(&self) -> bool {
        self.name == "dma-coherent" && self.data.is_empty()
    }
}

impl fmt::Display for Property<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            write!(f, "{}", self.name())
        } else if let Some(v) = self.as_address_cells() {
            write!(f, "#address-cells = <{:#x}>", v)
        } else if let Some(v) = self.as_size_cells() {
            write!(f, "#size-cells = <{:#x}>", v)
        } else if let Some(v) = self.as_interrupt_cells() {
            write!(f, "#interrupt-cells = <{:#x}>", v)
        } else if self.name() == "reg" {
            // reg 属性需要特殊处理，但我们没有 context 信息
            // 直接显示原始数据
            write!(f, "reg = ")?;
            format_bytes(f, &self.data())
        } else if let Some(s) = self.as_status() {
            write!(f, "status = \"{:?}\"", s)
        } else if let Some(p) = self.as_phandle() {
            write!(f, "phandle = {}", p)
        } else if let Some(p) = self.as_interrupt_parent() {
            write!(f, "interrupt-parent = {}", p)
        } else if let Some(s) = self.as_device_type() {
            write!(f, "device_type = \"{}\"", s)
        } else if let Some(iter) = self.as_compatible() {
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
        } else if let Some(iter) = self.as_clock_names() {
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
        } else if self.is_dma_coherent() {
            write!(f, "dma-coherent")
        } else if let Some(s) = self.as_str() {
            // 检查是否有多个字符串
            if self.data().iter().filter(|&&b| b == 0).count() > 1 {
                write!(f, "{} = ", self.name())?;
                let mut first = true;
                for s in self.as_str_iter() {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\"", s)?;
                    first = false;
                }
                Ok(())
            } else {
                write!(f, "{} = \"{}\"", self.name(), s)
            }
        } else if self.len() == 4 {
            // 单个 u32
            let v = u32::from_be_bytes(self.data().as_slice().try_into().unwrap());
            write!(f, "{} = <{:#x}>", self.name(), v)
        } else {
            // 原始字节
            write!(f, "{} = ", self.name())?;
            format_bytes(f, &self.data())
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
                    let len = match self.reader.read_u32() {
                        Some(b) => b,
                        None => {
                            self.handle_error(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            });
                            return None;
                        }
                    };

                    // 读取属性名偏移
                    let nameoff = match self.reader.read_u32() {
                        Some(b) => b,
                        None => {
                            self.handle_error(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            });
                            return None;
                        }
                    };

                    // 读取属性数据
                    let prop_data = if len > 0 {
                        match self.reader.read_bytes(len as _) {
                            Some(b) => b,
                            None => {
                                self.handle_error(FdtError::BufferTooSmall {
                                    pos: self.reader.position(),
                                });
                                return None;
                            }
                        }
                    } else {
                        Bytes::new(&[])
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

                    return Some(Property::new(name, prop_data));
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
