use crate::{
    Fdt, FdtError, Node, Token,
    data::Reader,
    node::{OneNodeIter, OneNodeState},
};

pub struct FdtIter<'a> {
    fdt: Fdt<'a>,
    reader: Reader<'a>,
    /// 当前正在处理的节点迭代器
    node_iter: Option<OneNodeIter<'a>>,
    has_err: bool,
    /// 当前层级深度
    level: usize,
}

impl<'a> FdtIter<'a> {
    pub fn new(fdt: Fdt<'a>) -> Self {
        let header = fdt.header();
        let struct_offset = header.off_dt_struct as usize;
        let reader = fdt.data.reader_at(struct_offset);
        Self {
            fdt,
            reader,
            node_iter: None,
            level: 0,
            has_err: false,
        }
    }
}

impl<'a> Iterator for FdtIter<'a> {
    type Item = Result<Node<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_err {
            return None;
        }

        loop {
            // 如果有正在处理的节点，继续处理它
            if let Some(ref mut node_iter) = self.node_iter {
                match node_iter.process() {
                    Ok(OneNodeState::ChildBegin) => {
                        // 遇到子节点，更新 reader 位置并清空当前节点迭代器
                        self.reader = node_iter.reader().clone();
                        self.node_iter = None;
                        // 继续循环，下一次会读取 BeginNode token
                    }
                    Ok(OneNodeState::End) => {
                        // 当前节点结束，更新 reader 并降低层级
                        self.reader = node_iter.reader().clone();
                        self.node_iter = None;
                        if self.level > 0 {
                            self.level -= 1;
                        }
                        // 继续循环处理下一个 token
                    }
                    Ok(OneNodeState::Processing) => {
                        // 不应该到达这里
                        continue;
                    }
                    Err(e) => {
                        self.has_err = true;
                        return Some(Err(e));
                    }
                }
                continue;
            }

            // 读取下一个 token
            match self.reader.read_token() {
                Ok(Token::BeginNode) => {
                    // 创建新的节点迭代器来处理这个节点
                    let mut node_iter = OneNodeIter::new(self.reader.clone(), self.level);

                    // 读取节点名称
                    match node_iter.read_node_name() {
                        Ok(node) => {
                            // 保存节点迭代器以便后续处理属性和子节点
                            self.node_iter = Some(node_iter);
                            // 增加层级
                            self.level += 1;
                            return Some(Ok(node));
                        }
                        Err(e) => {
                            self.has_err = true;
                            return Some(Err(e));
                        }
                    }
                }
                Ok(Token::EndNode) => {
                    // 顶层 EndNode，降低层级
                    if self.level > 0 {
                        self.level -= 1;
                    }
                    continue;
                }
                Ok(Token::End) => {
                    // 结构块结束
                    return None;
                }
                Ok(Token::Nop) => {
                    // 忽略 NOP
                    continue;
                }
                Ok(Token::Prop) | Ok(Token::Data(_)) => {
                    // 在顶层遇到属性或未知数据是错误的
                    self.has_err = true;
                    return Some(Err(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    }));
                }
                Err(e) => {
                    self.has_err = true;
                    return Some(Err(e));
                }
            }
        }
    }
}
