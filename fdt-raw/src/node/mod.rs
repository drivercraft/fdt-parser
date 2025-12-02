use core::ffi::CStr;

use crate::{
    FdtError, Token,
    data::{Bytes, Reader},
};

#[derive(Clone)]
pub struct Node<'a> {
    name: &'a str,
    data: Bytes<'a>,
    level: usize,
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
    state: OneNodeState,
    level: usize,
}

impl<'a> OneNodeIter<'a> {
    pub fn new(reader: Reader<'a>, level: usize) -> Self {
        Self {
            reader,
            state: OneNodeState::Processing,
            level,
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
            level: self.level,
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

    /// 处理节点内容，跳过属性，遇到子节点或结束时返回
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
                    // 跳过属性：读取 len 和 nameoff，然后跳过数据
                    let len_bytes = self.reader.read_bytes(4).ok_or(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    })?;
                    let len = u32::from_be_bytes(len_bytes.try_into().unwrap()) as usize;

                    // 跳过 nameoff (4 bytes)
                    let _ = self.reader.read_bytes(4).ok_or(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    })?;

                    // 跳过属性数据
                    if len > 0 {
                        let _ = self
                            .reader
                            .read_bytes(len)
                            .ok_or(FdtError::BufferTooSmall {
                                pos: self.reader.position(),
                            })?;
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
