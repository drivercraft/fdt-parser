use crate::{
    data::{Buffer, Raw},
    node::Node,
    FdtError, Header, ReserveEntry, Token,
};

#[derive(Clone)]
pub struct Fdt<'a> {
    header: Header,
    pub(crate) raw: Raw<'a>,
}

impl<'a> Fdt<'a> {
    /// Create a new `Fdt` from byte slice.
    pub fn from_bytes(data: &'a [u8]) -> Result<Fdt<'a>, FdtError> {
        let header = Header::from_bytes(data)?;
        if data.len() < header.totalsize as usize {
            return Err(FdtError::BufferTooSmall {
                pos: header.totalsize as usize,
            });
        }
        let buffer = Raw::new(data);
        Ok(Fdt {
            header,
            raw: buffer,
        })
    }

    /// Create a new `Fdt` from a raw pointer and size in bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// memory region of at least `size` bytes that contains a valid device tree
    /// blob.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Fdt<'a>, FdtError> {
        let header = Header::from_ptr(ptr)?;

        let raw = Raw::new(core::slice::from_raw_parts(ptr, header.totalsize as _));

        Ok(Fdt { header, raw })
    }

    /// Get a reference to the FDT header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn total_size(&self) -> usize {
        self.header.totalsize as usize
    }

    /// This field shall contain the physical ID of the system’s boot CPU. It shall be identical to the physical ID given in the
    /// reg property of that CPU node within the devicetree.
    pub fn boot_cpuid_phys(&self) -> u32 {
        self.header.boot_cpuid_phys
    }

    /// Get a reference to the underlying buffer.
    pub fn raw(&self) -> &'a [u8] {
        self.raw.raw()
    }

    /// Get the FDT version
    pub fn version(&self) -> u32 {
        self.header.version
    }

    pub fn memory_reservaion_blocks(&self) -> impl Iterator<Item = ReserveEntry> + 'a {
        let mut buffer = self.raw.buffer_at(self.header.off_mem_rsvmap as usize);

        core::iter::from_fn(move || {
            let address = buffer.take_u64().ok()?;
            let size = buffer.take_u64().ok()?;

            if address == 0 && size == 0 {
                return None;
            }

            Some(ReserveEntry { address, size })
        })
    }

    /// Alias for memory_reservaion_blocks for compatibility
    pub fn memory_reservation_block(&self) -> impl Iterator<Item = ReserveEntry> + 'a {
        self.memory_reservaion_blocks()
    }

    fn get_str(&self, offset: usize) -> Result<&'a str, FdtError> {
        let start = self.header.off_dt_strings as usize + offset;
        let mut buffer = self.raw.buffer_at(start);
        buffer.take_str()
    }

    /// 递归遍历所有节点，通过回调函数处理每个节点
    /// 参考 linux dtc 的实现方式
    pub fn walk_nodes_recursive<F>(&self, mut callback: F) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>, // 返回 true 继续遍历，false 停止
    {
        let mut buffer = self.raw.buffer_at(self.header.off_dt_struct as usize);
        self.walk_nodes_recursive_impl(&mut buffer, 0, &mut callback)
    }

    /// 递归遍历指定深度的节点
    pub fn walk_nodes_at_depth<F>(
        &self,
        target_depth: usize,
        mut callback: F,
    ) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>,
    {
        let mut buffer = self.raw.buffer_at(self.header.off_dt_struct as usize);
        self.walk_nodes_at_depth_impl(&mut buffer, 0, target_depth, &mut callback)
    }

    /// 查找并遍历特定节点的子节点
    pub fn walk_child_nodes<F>(
        &self,
        parent_name: &str,
        parent_level: usize,
        mut callback: F,
    ) -> Result<(), FdtError>
    where
        F: FnMut(&Node<'a>) -> Result<bool, FdtError>,
    {
        let mut buffer = self.raw.buffer_at(self.header.off_dt_struct as usize);
        self.walk_child_nodes_impl(&mut buffer, parent_name, parent_level, &mut callback)
    }

    /// 递归遍历的内部实现
    fn walk_nodes_recursive_impl<F>(
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

                    let node = Node::new(self, node_name, level, 0);

                    // 调用回调函数处理当前节点
                    let should_continue = callback(&node)?;
                    if !should_continue {
                        return Ok(());
                    }

                    // 递归处理子节点
                    self.walk_nodes_recursive_impl(buffer, level + 1, callback)?;
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
    fn walk_nodes_at_depth_impl<F>(
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
                        let node = Node::new(self, node_name, current_level, 0);
                        let should_continue = callback(&node)?;
                        if !should_continue {
                            return Ok(());
                        }
                    }

                    // 无论是否匹配深度，都需要递归处理以跳过子节点
                    self.walk_nodes_at_depth_impl(
                        buffer,
                        current_level + 1,
                        target_depth,
                        callback,
                    )?;
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
    fn walk_child_nodes_impl<F>(
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
                        let node = Node::new(self, node_name, current_level, 0);
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
