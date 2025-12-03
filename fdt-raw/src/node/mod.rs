use core::ffi::CStr;
use core::fmt;

use crate::{
    FdtError, Token,
    data::{Bytes, Reader},
};

mod prop;

pub use prop::{PropIter, Property, Reg, RegInfo, RegIter, StrIter, U32Iter};

/// 节点上下文，保存从父节点继承的信息
#[derive(Debug, Clone)]
pub struct NodeContext {
    /// 父节点的 #address-cells (用于解析当前节点的 reg)
    pub parent_address_cells: u8,
    /// 父节点的 #size-cells (用于解析当前节点的 reg)
    pub parent_size_cells: u8,
}

impl Default for NodeContext {
    fn default() -> Self {
        Self {
            // 默认值根据 DTSpec: 2 for address, 1 for size
            parent_address_cells: 2,
            parent_size_cells: 1,
        }
    }
}

#[derive(Clone)]
pub struct Node<'a> {
    name: &'a str,
    data: Bytes<'a>,
    strings: Bytes<'a>,
    level: usize,
    /// 当前节点的 #address-cells（用于子节点）
    pub address_cells: u8,
    /// 当前节点的 #size-cells（用于子节点）
    pub size_cells: u8,
    /// 继承的上下文（包含父节点的 cells 和累积的 ranges）
    pub context: NodeContext,
}

impl<'a> Node<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn level(&self) -> usize {
        self.level
    }

    /// 获取用于解析当前节点 reg 属性的 address cells
    pub fn reg_address_cells(&self) -> u8 {
        self.context.parent_address_cells
    }

    /// 获取用于解析当前节点 reg 属性的 size cells
    pub fn reg_size_cells(&self) -> u8 {
        self.context.parent_size_cells
    }

    /// 为子节点创建上下文
    pub(crate) fn create_child_context(&self) -> NodeContext {
        NodeContext {
            parent_address_cells: self.address_cells,
            parent_size_cells: self.size_cells,
        }
    }

    /// 获取节点属性迭代器
    pub fn properties(&self) -> PropIter<'a> {
        PropIter::new(
            self.data.reader(),
            self.strings.clone(),
            self.context.clone(),
        )
    }

    /// 查找并解析 reg 属性，返回 Reg 迭代器
    pub fn reg(&self) -> Option<Reg<'a>> {
        for prop in self.properties() {
            if let Property::Reg(reg) = prop {
                return Some(reg);
            }
        }
        None
    }

    /// 查找并解析 reg 属性，返回所有 RegInfo 条目
    pub fn reg_array<const N: usize>(&self) -> heapless::Vec<RegInfo, N> {
        let mut result = heapless::Vec::new();
        if let Some(reg) = self.reg() {
            for info in reg.iter() {
                if result.push(info).is_err() {
                    break; // 数组已满
                }
            }
        }
        result
    }
}

/// 写入缩进
fn write_indent(f: &mut fmt::Formatter<'_>, count: usize, ch: &str) -> fmt::Result {
    for _ in 0..count {
        write!(f, "{}", ch)?;
    }
    Ok(())
}

impl fmt::Display for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_indent(f, self.level, "    ")?;
        let name = if self.name.is_empty() { "/" } else { self.name };

        writeln!(f, "{} {{", name)?;
        for prop in self.properties() {
            write_indent(f, self.level + 1, "    ")?;
            writeln!(f, "{};", prop)?;
        }
        write_indent(f, self.level, "    ")?;
        write!(f, "}}")
    }
}

/// 解析属性时提取的关键信息
#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedProps {
    pub address_cells: Option<u8>,
    pub size_cells: Option<u8>,
}

/// 单节点迭代状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OneNodeState {
    /// 正在处理当前节点
    Processing,
    /// 遇到子节点的 BeginNode，需要回溯
    ChildBegin,
    /// 遇到 EndNode，当前节点处理完成
    End,
}

/// An iterator over a single node's content.
/// When encountering a child's BeginNode, it backtracks and signals FdtIter to handle it.
pub(crate) struct OneNodeIter<'a> {
    reader: Reader<'a>,
    strings: Bytes<'a>,
    state: OneNodeState,
    level: usize,
    context: NodeContext,
    parsed_props: ParsedProps,
}

impl<'a> OneNodeIter<'a> {
    pub fn new(reader: Reader<'a>, strings: Bytes<'a>, level: usize, context: NodeContext) -> Self {
        Self {
            reader,
            strings,
            state: OneNodeState::Processing,
            level,
            context,
            parsed_props: ParsedProps::default(),
        }
    }

    pub fn reader(&self) -> &Reader<'a> {
        &self.reader
    }

    pub fn parsed_props(&self) -> &ParsedProps {
        &self.parsed_props
    }

    /// 读取节点名称（在 BeginNode token 之后调用）
    pub fn read_node_name(&mut self) -> Result<Node<'a>, FdtError> {
        // 读取以 null 结尾的名称字符串
        let name = self.read_cstr()?;

        // 对齐到 4 字节边界
        self.align4();

        let data = self.reader.remain();

        Ok(Node {
            name,
            data,
            strings: self.strings.clone(),
            level: self.level,
            // 默认值，会在 process() 中更新
            address_cells: 2,
            size_cells: 1,
            context: self.context.clone(),
        })
    }

    fn read_cstr(&mut self) -> Result<&'a str, FdtError> {
        let bytes = self.reader.remain();
        let cstr = CStr::from_bytes_until_nul(bytes.as_slice())?;
        let s = cstr.to_str()?;
        // 跳过字符串内容 + null 终止符
        let _ = self.reader.read_bytes(s.len() + 1);
        Ok(s)
    }

    fn align4(&mut self) {
        let pos = self.reader.position();
        let aligned = (pos + 3) & !3;
        let skip = aligned - pos;
        if skip > 0 {
            let _ = self.reader.read_bytes(skip);
        }
    }

    /// 从 strings block 读取属性名
    fn read_prop_name(&self, nameoff: u32) -> Result<&'a str, FdtError> {
        let bytes = self.strings.slice(nameoff as usize..self.strings.len());
        let cstr = CStr::from_bytes_until_nul(bytes.as_slice())?;
        Ok(cstr.to_str()?)
    }

    /// 读取 u32 从大端字节
    fn read_u32_be(data: &[u8], offset: usize) -> u64 {
        u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as u64
    }

    /// 处理节点内容，解析关键属性，遇到子节点或结束时返回
    pub fn process(&mut self) -> Result<OneNodeState, FdtError> {
        loop {
            let token = self.reader.read_token()?;
            match token {
                Token::BeginNode => {
                    // 遇到子节点，回溯 token 并返回
                    self.reader.backtrack(4);
                    self.state = OneNodeState::ChildBegin;
                    return Ok(OneNodeState::ChildBegin);
                }
                Token::EndNode => {
                    self.state = OneNodeState::End;
                    return Ok(OneNodeState::End);
                }
                Token::Prop => {
                    // 读取属性：len 和 nameoff
                    let len_bytes = self.reader.read_bytes(4).ok_or(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    })?;
                    let len = u32::from_be_bytes(len_bytes.try_into().unwrap()) as usize;

                    let nameoff_bytes =
                        self.reader.read_bytes(4).ok_or(FdtError::BufferTooSmall {
                            pos: self.reader.position(),
                        })?;
                    let nameoff = u32::from_be_bytes(nameoff_bytes.try_into().unwrap());

                    // 读取属性数据
                    let prop_data = if len > 0 {
                        self.reader
                            .read_bytes(len)
                            .ok_or(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            })?
                    } else {
                        &[]
                    };

                    // 解析关键属性
                    if let Ok(prop_name) = self.read_prop_name(nameoff) {
                        match prop_name {
                            "#address-cells" if len == 4 => {
                                self.parsed_props.address_cells =
                                    Some(Self::read_u32_be(prop_data, 0) as u8);
                            }
                            "#size-cells" if len == 4 => {
                                self.parsed_props.size_cells =
                                    Some(Self::read_u32_be(prop_data, 0) as u8);
                            }
                            _ => {}
                        }
                    }

                    // 对齐到 4 字节边界
                    self.align4();
                }
                Token::Nop => {
                    // 忽略 NOP
                }
                Token::End => {
                    // 结构块结束
                    self.state = OneNodeState::End;
                    return Ok(OneNodeState::End);
                }
                Token::Data(_) => {
                    // 非法 token
                    return Err(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    });
                }
            }
        }
    }
}
