use crate::{data::Buffer, node::Node, Fdt, FdtError, Token};

/// Walker 结构体，包含所有的遍历相关逻辑
pub struct Walker<'a> {
    fdt: Fdt<'a>,
}

impl<'a> Walker<'a> {
    /// 创建新的Walker实例
    pub fn new(fdt: Fdt<'a>) -> Self {
        Self { fdt }
    }

    /// 获取FDT引用
    pub fn fdt(&self) -> &Fdt<'a> {
        &self.fdt
    }

    fn buffer(&self) -> Buffer<'a> {
        self.fdt
            .raw
            .buffer_at(self.fdt.header().off_dt_struct as usize)
    }

    /// 递归遍历所有节点，通过回调函数处理每个节点
    /// 参考 linux dtc 的实现方式
    pub fn walk_all<F>(&self, mut callback: F) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>, // 返回 true 继续遍历，false 停止
    {
        self.walk_recursive_impl(&mut self.buffer(), 0, &mut callback)
    }

    /// 递归遍历指定深度的节点
    pub fn walk_at_depth<F>(&self, target_depth: usize, mut callback: F) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>,
    {
        self.walk_at_depth_impl(&mut self.buffer(), 0, target_depth, &mut callback)
    }

    /// 查找并遍历特定节点的子节点
    pub fn walk_children<F>(
        &self,
        parent_name: &str,
        parent_level: usize,
        mut callback: F,
    ) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>,
    {
        self.walk_children_impl(&mut self.buffer(), parent_name, parent_level, &mut callback)
    }

    /// 查找特定名称的节点
    pub fn find_node<F>(&self, node_name: &str, mut callback: F) -> Result<bool, FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<(), FdtError>,
    {
        let mut found = false;
        self.walk_all(|node| {
            if node.name() == node_name {
                callback(node)?;
                found = true;
                Ok(false) // 找到后停止遍历
            } else {
                Ok(true) // 继续遍历
            }
        })?;
        Ok(found)
    }

    /// 查找所有匹配条件的节点
    pub fn find_nodes<F, P>(&self, mut predicate: P, mut callback: F) -> Result<usize, FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<(), FdtError>,
        P: FnMut(&Node<'a>) -> bool,
    {
        let mut count = 0;
        self.walk_all(|node| {
            if predicate(node) {
                callback(node)?;
                count += 1;
            }
            Ok(true) // 继续遍历所有节点
        })?;
        Ok(count)
    }

    /// 获取节点总数
    pub fn count_nodes(&self) -> Result<usize, FdtError> {
        let mut count = 0;
        self.walk_all(|_| {
            count += 1;
            Ok(true)
        })?;
        Ok(count)
    }

    /// 获取指定深度的节点数量
    pub fn count_nodes_at_depth(&self, target_depth: usize) -> Result<usize, FdtError> {
        let mut count = 0;
        self.walk_at_depth(target_depth, |_| {
            count += 1;
            Ok(true)
        })?;
        Ok(count)
    }

    /// 遍历节点直到满足条件
    pub fn walk_until<F, P>(&self, mut predicate: P, mut callback: F) -> Result<bool, FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<(), FdtError>,
        P: FnMut(&Node<'a>) -> bool,
    {
        let mut found = false;
        self.walk_all(|node| {
            callback(node)?;
            if predicate(node) {
                found = true;
                Ok(false) // 满足条件后停止
            } else {
                Ok(true) // 继续遍历
            }
        })?;
        Ok(found)
    }

    /// 遍历指定节点的所有后代
    pub fn walk_descendants<F>(
        &self,
        ancestor_name: &str,
        ancestor_level: usize,
        mut callback: F,
    ) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>,
    {
        let mut in_subtree = false;
        let mut subtree_level = 0;

        self.walk_all(|node| {
            if !in_subtree && node.name() == ancestor_name && node.level() == ancestor_level {
                in_subtree = true;
                subtree_level = node.level();
                return Ok(true); // 找到祖先节点，开始遍历子树
            }

            if in_subtree {
                if node.level() > subtree_level {
                    // 这是祖先节点的后代
                    return callback(node);
                } else {
                    // 已经退出子树
                    return Ok(false);
                }
            }

            Ok(true)
        })
    }

    /// 批量操作：对匹配的节点执行操作
    pub fn batch_operation<F, P>(
        &self,
        mut predicate: P,
        mut operation: F,
    ) -> Result<usize, FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<(), FdtError>,
        P: FnMut(&Node<'a>) -> bool,
    {
        let mut processed = 0;
        self.walk_all(|node| {
            if predicate(node) {
                operation(node)?;
                processed += 1;
            }
            Ok(true)
        })?;
        Ok(processed)
    }

    /// 统计各层级的节点数量（固定大小数组，最大支持32层）
    pub fn count_by_depth(&self, max_depth: usize) -> Result<[usize; 32], FdtError> {
        let mut counts = [0usize; 32];
        let actual_max = core::cmp::min(max_depth, 31);

        self.walk_all(|node| {
            if node.level() <= actual_max {
                counts[node.level()] += 1;
            }
            Ok(true)
        })?;
        Ok(counts)
    }

    /// 递归遍历的内部实现
    fn walk_recursive_impl<F>(
        &self,
        buffer: &mut Buffer<'a>,
        level: usize,
        callback: &mut F,
    ) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>,
    {
        loop {
            let token = match buffer.take_token() {
                Ok(token) => token,
                Err(_) => return Ok(()), // 到达缓冲区末尾
            };

            match token {
                Token::BeginNode => {
                    // 读取节点名称
                    let node_name = match buffer.take_str() {
                        Ok(name) => name,
                        Err(_) => return Ok(()),
                    };

                    let node = Node::new(&self.fdt, node_name, level, 0);

                    // 调用回调函数处理当前节点
                    let should_continue = callback(&node)?;
                    if !should_continue {
                        return Ok(());
                    }

                    // 递归处理子节点
                    self.walk_recursive_impl(buffer, level + 1, callback)?;
                }
                Token::EndNode => {
                    // 当前层级结束，返回上一层
                    return Ok(());
                }
                Token::Prop => {
                    // 跳过属性：读取长度、名称偏移和数据
                    if let Ok(len) = buffer.take_u32() {
                        if buffer.take_u32().is_ok() {
                            let aligned_len = (len + 3) & !3;
                            let _ = buffer.take(aligned_len as usize);
                        }
                    }
                }
                Token::Nop => {
                    // 跳过 NOP token
                    continue;
                }
                Token::End => {
                    // 设备树结束
                    return Ok(());
                }
                Token::Data => {
                    // 在正常的 FDT 结构中不应该出现这种情况
                    continue;
                }
            }
        }
    }

    /// 遍历指定深度节点的内部实现
    fn walk_at_depth_impl<F>(
        &self,
        buffer: &mut Buffer<'a>,
        current_level: usize,
        target_depth: usize,
        callback: &mut F,
    ) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>,
    {
        loop {
            let token = match buffer.take_token() {
                Ok(token) => token,
                Err(_) => return Ok(()),
            };

            match token {
                Token::BeginNode => {
                    let node_name = match buffer.take_str() {
                        Ok(name) => name,
                        Err(_) => return Ok(()),
                    };

                    if current_level == target_depth {
                        let node = Node::new(&self.fdt, node_name, current_level, 0);
                        let should_continue = callback(&node)?;
                        if !should_continue {
                            return Ok(());
                        }
                    }

                    // 无论是否匹配深度，都需要递归处理以跳过子节点
                    self.walk_at_depth_impl(buffer, current_level + 1, target_depth, callback)?;
                }
                Token::EndNode => {
                    return Ok(());
                }
                Token::Prop => {
                    if let Ok(len) = buffer.take_u32() {
                        if buffer.take_u32().is_ok() {
                            let aligned_len = (len + 3) & !3;
                            let _ = buffer.take(aligned_len as usize);
                        }
                    }
                }
                Token::Nop => {
                    continue;
                }
                Token::End => {
                    return Ok(());
                }
                Token::Data => {
                    continue;
                }
            }
        }
    }

    /// 遍历子节点的内部实现
    fn walk_children_impl<F>(
        &self,
        buffer: &mut Buffer<'a>,
        parent_name: &str,
        parent_level: usize,
        callback: &mut F,
    ) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>,
    {
        let mut current_level = 0;
        let mut found_parent = false;

        loop {
            let token = match buffer.take_token() {
                Ok(token) => token,
                Err(_) => return Ok(()),
            };

            match token {
                Token::BeginNode => {
                    let node_name = match buffer.take_str() {
                        Ok(name) => name,
                        Err(_) => return Ok(()),
                    };

                    if !found_parent && current_level == parent_level && node_name == parent_name {
                        found_parent = true;
                        current_level += 1;
                        continue;
                    }

                    if found_parent && current_level == parent_level + 1 {
                        // 这是目标父节点的直接子节点
                        let node = Node::new(&self.fdt, node_name, current_level, 0);
                        let should_continue = callback(&node)?;
                        if !should_continue {
                            return Ok(());
                        }
                        // 跳过这个子节点的子树
                        self.skip_subtree(buffer)?;
                    } else if found_parent {
                        // 在目标父节点内部，但不是直接子节点，跳过整个子树
                        self.skip_subtree(buffer)?;
                    } else {
                        // 还没找到父节点，递归继续查找
                        current_level += 1;
                    }
                }
                Token::EndNode => {
                    if found_parent && current_level == parent_level + 1 {
                        // 父节点结束，完成收集
                        return Ok(());
                    }
                    current_level = current_level.saturating_sub(1);
                }
                Token::Prop => {
                    if let Ok(len) = buffer.take_u32() {
                        if buffer.take_u32().is_ok() {
                            let aligned_len = (len + 3) & !3;
                            let _ = buffer.take(aligned_len as usize);
                        }
                    }
                }
                Token::Nop => {
                    continue;
                }
                Token::End => {
                    return Ok(());
                }
                Token::Data => {
                    continue;
                }
            }
        }
    }

    /// 跳过整个子树
    fn skip_subtree(&self, buffer: &mut Buffer<'a>) -> Result<(), FdtError> {
        let mut depth = 1;

        while depth > 0 {
            let token = match buffer.take_token() {
                Ok(token) => token,
                Err(_) => return Ok(()),
            };

            match token {
                Token::BeginNode => {
                    let _ = buffer.take_str(); // 跳过节点名
                    depth += 1;
                }
                Token::EndNode => {
                    depth -= 1;
                }
                Token::Prop => {
                    if let Ok(len) = buffer.take_u32() {
                        if buffer.take_u32().is_ok() {
                            let aligned_len = (len + 3) & !3;
                            let _ = buffer.take(aligned_len as usize);
                        }
                    }
                }
                Token::Nop => {
                    continue;
                }
                Token::End => {
                    return Ok(());
                }
                Token::Data => {
                    continue;
                }
            }
        }

        Ok(())
    }
}
