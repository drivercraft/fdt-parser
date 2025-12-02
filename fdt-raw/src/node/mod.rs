use core::ffi::CStr;
use core::fmt;

use crate::{
    FdtError, Token,
    data::{Bytes, Reader},
};

mod prop;

pub use prop::{PropIter, Property, StrIter, U32Iter};

/// 地址范围转换条目
/// 用于将子地址空间映射到父地址空间
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeEntry {
    /// 子地址空间中的起始地址
    pub child_bus_addr: u64,
    /// 父地址空间中的起始地址
    pub parent_bus_addr: u64,
    /// 范围长度
    pub length: u64,
}

impl RangeEntry {
    /// 将子地址转换为父地址
    pub fn translate(&self, child_addr: u64) -> Option<u64> {
        if child_addr >= self.child_bus_addr
            && child_addr < self.child_bus_addr.saturating_add(self.length)
        {
            Some(self.parent_bus_addr + (child_addr - self.child_bus_addr))
        } else {
            None
        }
    }
}

/// 节点上下文，保存从父节点继承的信息
#[derive(Debug, Clone)]
pub struct NodeContext {
    /// 父节点的 #address-cells (用于解析当前节点的 reg)
    pub parent_address_cells: u8,
    /// 父节点的 #size-cells (用于解析当前节点的 reg)
    pub parent_size_cells: u8,
    /// 累积的地址转换范围（从根节点到当前节点的父节点）
    pub ranges: heapless::Vec<RangeEntry, 16>,
}

impl Default for NodeContext {
    fn default() -> Self {
        Self {
            // 默认值根据 DTSpec: 2 for address, 1 for size
            parent_address_cells: 2,
            parent_size_cells: 1,
            ranges: heapless::Vec::new(),
        }
    }
}

impl NodeContext {
    /// 将子地址通过所有 ranges 转换为根地址
    pub fn translate_address(&self, child_addr: u64) -> u64 {
        let mut addr = child_addr;
        for range in self.ranges.iter().rev() {
            if let Some(translated) = range.translate(addr) {
                addr = translated;
            }
        }
        addr
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

    pub(crate) fn data(&self) -> &Bytes<'a> {
        &self.data
    }

    /// 获取用于解析当前节点 reg 属性的 address cells
    pub fn reg_address_cells(&self) -> u8 {
        self.context.parent_address_cells
    }

    /// 获取用于解析当前节点 reg 属性的 size cells
    pub fn reg_size_cells(&self) -> u8 {
        self.context.parent_size_cells
    }

    /// 将节点地址转换为根地址空间
    pub fn translate_address(&self, addr: u64) -> u64 {
        self.context.translate_address(addr)
    }

    /// 为子节点创建上下文
    pub(crate) fn create_child_context(&self, child_ranges: &[RangeEntry]) -> NodeContext {
        let mut ctx = NodeContext {
            parent_address_cells: self.address_cells,
            parent_size_cells: self.size_cells,
            ranges: self.context.ranges.clone(),
        };
        // 添加当前节点的 ranges 到累积列表
        for range in child_ranges {
            let _ = ctx.ranges.push(*range);
        }
        ctx
    }

    /// 获取节点属性迭代器
    pub fn properties(&self) -> PropIter<'a> {
        PropIter::new(self.data.reader(), self.strings.clone())
    }
}

impl fmt::Display for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let indent = "    ".repeat(self.level);
        let name = if self.name.is_empty() { "/" } else { self.name };

        writeln!(f, "{}{} {{", indent, name)?;
        for prop in self.properties() {
            writeln!(f, "{}    {};", indent, prop)?;
        }
        write!(f, "{}}}", indent)
    }
}

/// 解析属性时提取的关键信息
#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedProps {
    pub address_cells: Option<u8>,
    pub size_cells: Option<u8>,
    pub ranges: heapless::Vec<RangeEntry, 16>,
    pub ranges_empty: bool, // ranges 属性存在但为空（1:1 映射）
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

    pub fn state(&self) -> OneNodeState {
        self.state
    }

    pub fn reader(&self) -> &Reader<'a> {
        &self.reader
    }

    pub fn into_reader(self) -> Reader<'a> {
        self.reader
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

    /// 读取 u64 从大端字节
    fn read_u64_be(data: &[u8], offset: usize) -> u64 {
        u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap())
    }

    /// 根据 cells 数量读取值
    fn read_cells(data: &[u8], offset: usize, cells: u8) -> u64 {
        match cells {
            1 => Self::read_u32_be(data, offset),
            2 => Self::read_u64_be(data, offset),
            _ => Self::read_u32_be(data, offset),
        }
    }

    /// 解析 ranges 属性
    fn parse_ranges(&mut self, data: &[u8]) {
        if data.is_empty() {
            // 空 ranges 表示 1:1 映射
            self.parsed_props.ranges_empty = true;
            return;
        }

        // ranges 格式: child_bus_addr, parent_bus_addr, length
        // 使用当前节点的 address_cells 和 size_cells (用于 child 和 length)
        // 使用父节点的 address_cells (用于 parent)
        let child_addr_cells = self.parsed_props.address_cells.unwrap_or(2);
        let parent_addr_cells = self.context.parent_address_cells;
        let size_cells = self.parsed_props.size_cells.unwrap_or(1);

        let entry_size =
            (child_addr_cells as usize + parent_addr_cells as usize + size_cells as usize) * 4;

        if entry_size == 0 {
            return;
        }

        let mut offset = 0;
        while offset + entry_size <= data.len() {
            let child_bus_addr = Self::read_cells(data, offset, child_addr_cells);
            offset += child_addr_cells as usize * 4;

            let parent_bus_addr = Self::read_cells(data, offset, parent_addr_cells);
            offset += parent_addr_cells as usize * 4;

            let length = Self::read_cells(data, offset, size_cells);
            offset += size_cells as usize * 4;

            let _ = self.parsed_props.ranges.push(RangeEntry {
                child_bus_addr,
                parent_bus_addr,
                length,
            });
        }
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
                            "ranges" => {
                                self.parse_ranges(prop_data);
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
