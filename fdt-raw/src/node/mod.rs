use core::fmt;
use core::ops::Deref;
use core::{ffi::CStr, fmt::Debug};

use crate::{
    FdtError, Token,
    data::{Bytes, Reader},
};

mod chosen;
mod memory;
mod prop;

pub use chosen::Chosen;
pub use memory::{Memory, MemoryRegion};
pub use prop::{PropIter, Property, RangeInfo, RegInfo, RegIter, VecRange};

/// 节点上下文，保存从父节点继承的信息
#[derive(Clone)]
pub(crate) struct NodeContext {
    /// 父节点的 #address-cells (用于解析当前节点的 reg)
    pub address_cells: u8,
    /// 父节点的 #size-cells (用于解析当前节点的 reg)
    pub size_cells: u8,
}

impl Default for NodeContext {
    fn default() -> Self {
        NodeContext {
            address_cells: 2,
            size_cells: 1,
        }
    }
}

/// 基础节点结构
#[derive(Clone)]
pub struct NodeBase<'a> {
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

impl<'a> NodeBase<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn level(&self) -> usize {
        self.level
    }

    /// 获取节点属性迭代器
    pub fn properties(&self) -> PropIter<'a> {
        PropIter::new(self.data.reader(), self.strings.clone())
    }

    /// 查找指定名称的属性
    pub fn find_property(&self, name: &str) -> Option<Property<'a>> {
        self.properties().find(|p| p.name() == name)
    }

    /// 查找指定名称的字符串属性
    pub fn find_property_str(&self, name: &str) -> Option<&'a str> {
        let prop = self.find_property(name)?;

        // 否则作为普通字符串处理
        prop.as_str()
    }

    /// 查找并解析 reg 属性，返回 Reg 迭代器
    pub fn reg(&self) -> Option<RegIter<'a>> {
        let prop = self.find_property("reg")?;
        Some(RegIter::new(
            prop.data().reader(),
            self.context.address_cells,
            self.context.size_cells,
        ))
    }

    /// 查找并解析 reg 属性，返回所有 RegInfo 条目
    pub fn reg_array<const N: usize>(&self) -> heapless::Vec<RegInfo, N> {
        let mut result = heapless::Vec::new();
        if let Some(reg) = self.reg() {
            for info in reg {
                if result.push(info).is_err() {
                    break; // 数组已满
                }
            }
        }
        result
    }

    /// 检查是否是 chosen 节点
    fn is_chosen(&self) -> bool {
        self.name == "chosen"
    }

    /// 检查是否是 memory 节点
    fn is_memory(&self) -> bool {
        self.name.starts_with("memory")
    }

    pub fn ranges(&self) -> Option<VecRange<'a>> {
        let prop = self.find_property("ranges")?;
        Some(VecRange::new(
            self.address_cells as usize,
            self.context.address_cells as usize,
            self.context.size_cells as usize,
            prop.data(),
        ))
    }
}

/// 写入缩进
fn write_indent(f: &mut fmt::Formatter<'_>, count: usize, ch: &str) -> fmt::Result {
    for _ in 0..count {
        write!(f, "{}", ch)?;
    }
    Ok(())
}

impl fmt::Display for NodeBase<'_> {
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

// ============================================================================
// Node 枚举：支持特化节点类型
// ============================================================================

/// 节点枚举，支持 General、Chosen、Memory 等特化类型
#[derive(Clone)]
pub enum Node<'a> {
    /// 通用节点
    General(NodeBase<'a>),
    /// Chosen 节点，包含启动参数
    Chosen(Chosen<'a>),
    /// Memory 节点，描述物理内存布局
    Memory(Memory<'a>),
}

impl<'a> From<NodeBase<'a>> for Node<'a> {
    fn from(node: NodeBase<'a>) -> Self {
        if node.is_chosen() {
            Node::Chosen(Chosen::new(node))
        } else if node.is_memory() {
            Node::Memory(Memory::new(node))
        } else {
            Node::General(node)
        }
    }
}

impl<'a> Deref for Node<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            Node::General(n) => n,
            Node::Chosen(c) => c.deref(),
            Node::Memory(m) => m.deref(),
        }
    }
}

impl fmt::Display for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::General(n) => f.debug_tuple("General").field(&n.name()).finish(),
            Node::Chosen(c) => c.fmt(f),
            Node::Memory(m) => m.fmt(f),
        }
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
    pub fn read_node_name(&mut self) -> Result<NodeBase<'a>, FdtError> {
        // 读取以 null 结尾的名称字符串
        let name = self.read_cstr()?;

        // 对齐到 4 字节边界
        self.align4();

        let data = self.reader.remain();

        Ok(NodeBase {
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
                    let len = self.reader.read_u32().ok_or(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    })? as usize;

                    let nameoff = self.reader.read_u32().ok_or(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    })?;

                    // 读取属性数据
                    let prop_data = if len > 0 {
                        self.reader
                            .read_bytes(len)
                            .ok_or(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            })?
                    } else {
                        Bytes::new(&[])
                    };

                    // 解析关键属性
                    if let Ok(prop_name) = self.read_prop_name(nameoff) {
                        match prop_name {
                            "#address-cells" if len == 4 => {
                                self.parsed_props.address_cells =
                                    Some(Self::read_u32_be(&prop_data, 0) as u8);
                            }
                            "#size-cells" if len == 4 => {
                                self.parsed_props.size_cells =
                                    Some(Self::read_u32_be(&prop_data, 0) as u8);
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
