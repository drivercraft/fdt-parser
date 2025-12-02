use log::error;

use crate::{
    Fdt, FdtError, Node, Token,
    data::{Bytes, Reader},
    node::{NodeContext, OneNodeIter, OneNodeState},
};

pub struct FdtIter<'a> {
    fdt: Fdt<'a>,
    reader: Reader<'a>,
    strings: Bytes<'a>,
    /// 当前正在处理的节点迭代器
    node_iter: Option<OneNodeIter<'a>>,
    /// 是否已终止（出错或结束）
    finished: bool,
    /// 当前层级深度
    level: usize,
    /// 上下文栈，栈顶为当前上下文
    context_stack: heapless::Vec<NodeContext, 32>,
}

impl<'a> FdtIter<'a> {
    pub fn new(fdt: Fdt<'a>) -> Self {
        let header = fdt.header();
        let struct_offset = header.off_dt_struct as usize;
        let strings_offset = header.off_dt_strings as usize;
        let strings_size = header.size_dt_strings as usize;

        let reader = fdt.data.reader_at(struct_offset);
        let strings = fdt
            .data
            .slice(strings_offset..strings_offset + strings_size);

        // 初始化上下文栈，压入默认上下文
        let mut context_stack = heapless::Vec::new();
        let _ = context_stack.push(NodeContext::default());

        Self {
            fdt,
            reader,
            strings,
            node_iter: None,
            level: 0,
            finished: false,
            context_stack,
        }
    }

    /// 获取当前上下文（栈顶）
    #[inline]
    fn current_context(&self) -> &NodeContext {
        // 栈永远不为空，因为初始化时压入了默认上下文
        self.context_stack.last().unwrap()
    }

    /// 处理错误：输出错误日志并终止迭代
    fn handle_error(&mut self, err: FdtError) {
        error!("FDT parse error: {}", err);
        self.finished = true;
    }
}

impl<'a> Iterator for FdtIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
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
                            // 弹出栈顶，恢复父节点上下文
                            self.context_stack.pop();
                        }
                        // 继续循环处理下一个 token
                    }
                    Ok(OneNodeState::Processing) => {
                        // 不应该到达这里
                        continue;
                    }
                    Err(e) => {
                        self.handle_error(e);
                        return None;
                    }
                }
                continue;
            }

            // 读取下一个 token
            match self.reader.read_token() {
                Ok(Token::BeginNode) => {
                    // 创建新的节点迭代器来处理这个节点
                    let mut node_iter = OneNodeIter::new(
                        self.reader.clone(),
                        self.strings.clone(),
                        self.level,
                        self.current_context().clone(),
                    );

                    // 读取节点名称
                    match node_iter.read_node_name() {
                        Ok(mut node) => {
                            // 先处理节点属性以获取 address-cells, size-cells, ranges
                            match node_iter.process() {
                                Ok(state) => {
                                    let props = node_iter.parsed_props();

                                    // 更新节点的 cells
                                    node.address_cells = props.address_cells.unwrap_or(2);
                                    node.size_cells = props.size_cells.unwrap_or(1);

                                    // 根据状态决定下一步动作
                                    match state {
                                        OneNodeState::ChildBegin => {
                                            // 有子节点，压入子节点上下文
                                            let child_context =
                                                node.create_child_context(&props.ranges);
                                            let _ = self.context_stack.push(child_context);

                                            // 有子节点，更新 reader 位置
                                            self.reader = node_iter.reader().clone();
                                            // 增加层级（节点有子节点）
                                            self.level += 1;
                                        }
                                        OneNodeState::End => {
                                            // 节点已结束（没有子节点），更新 reader
                                            self.reader = node_iter.reader().clone();
                                            // 不压栈，不更新上下文，因为节点没有子节点
                                            // 不增加层级，因为节点已经关闭
                                        }
                                        OneNodeState::Processing => {
                                            // 不应该到达这里，因为 process() 应该总是返回 ChildBegin 或 End
                                            self.node_iter = Some(node_iter);
                                            self.level += 1;
                                        }
                                    }

                                    return Some(node);
                                }
                                Err(e) => {
                                    self.handle_error(e);
                                    return None;
                                }
                            }
                        }
                        Err(e) => {
                            self.handle_error(e);
                            return None;
                        }
                    }
                }
                Ok(Token::EndNode) => {
                    // 顶层 EndNode，降低层级
                    if self.level > 0 {
                        self.level -= 1;
                        // 弹出栈顶，恢复父节点上下文
                        self.context_stack.pop();
                    }
                    continue;
                }
                Ok(Token::End) => {
                    // 结构块结束
                    self.finished = true;
                    return None;
                }
                Ok(Token::Nop) => {
                    // 忽略 NOP
                    continue;
                }
                Ok(Token::Prop) | Ok(Token::Data(_)) => {
                    // 在顶层遇到属性或未知数据是错误的
                    self.handle_error(FdtError::BufferTooSmall {
                        pos: self.reader.position(),
                    });
                    return None;
                }
                Err(e) => {
                    self.handle_error(e);
                    return None;
                }
            }
        }
    }
}
