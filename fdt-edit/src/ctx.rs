use alloc::{string::String, vec::Vec};
use fdt_raw::{Phandle, Status};

use crate::{Node, NodeOp, RangesEntry, prop::PropertyKind};

/// 遍历上下文，存储从根到当前节点的父节点引用栈
#[derive(Clone, Debug)]
pub struct FdtContext<'a> {
    /// 父节点引用栈（从根节点到当前节点的父节点）
    /// 栈底是根节点，栈顶是当前节点的直接父节点
    pub parents: Vec<&'a Node>,
    /// 当前节点的完整路径
    pub current_path: String,
}

impl<'a> Default for FdtContext<'a> {
    fn default() -> Self {
        Self {
            parents: Vec::new(),
            current_path: String::new(),
        }
    }
}

impl<'a> FdtContext<'a> {
    /// 创建新的上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建用于根节点的上下文
    pub fn for_root() -> Self {
        Self::default()
    }

    /// 获取当前深度（父节点数量 + 1）
    pub fn depth(&self) -> usize {
        self.parents.len() + 1
    }

    /// 获取直接父节点
    pub fn parent(&self) -> Option<&'a Node> {
        self.parents.last().copied()
    }

    /// 获取父节点的 #address-cells
    /// 优先从直接父节点获取，否则返回默认值 2
    pub fn parent_address_cells(&self) -> u8 {
        self.parent().and_then(|p| p.address_cells()).unwrap_or(2)
    }

    /// 获取父节点的 #size-cells
    /// 优先从直接父节点获取，否则返回默认值 1
    pub fn parent_size_cells(&self) -> u8 {
        self.parent().and_then(|p| p.size_cells()).unwrap_or(1)
    }

    /// 查找中断父节点 phandle
    /// 从当前父节点向上查找，返回最近的 interrupt-parent
    pub fn interrupt_parent(&self) -> Option<Phandle> {
        for parent in self.parents.iter().rev() {
            if let Some(phandle) = parent.interrupt_parent() {
                return Some(phandle);
            }
        }
        None
    }

    /// 检查节点是否被禁用
    /// 检查父节点栈中是否有任何节点被禁用
    pub fn is_disabled(&self) -> bool {
        for parent in &self.parents {
            if matches!(parent.status(), Some(Status::Disabled)) {
                return true;
            }
        }
        false
    }

    /// 解析当前路径上所有父节点的 ranges，用于地址转换
    /// 返回从根到父节点的 ranges 栈
    pub fn collect_ranges(&self) -> Vec<Vec<RangesEntry>> {
        let mut ranges_stack = Vec::new();
        let mut prev_address_cells: u8 = 2; // 根节点默认

        for parent in &self.parents {
            if let Some(ranges) = parse_ranges_for_node(parent, prev_address_cells) {
                ranges_stack.push(ranges);
            }
            // 更新 address cells 为当前节点的值，供下一级使用
            prev_address_cells = parent.address_cells().unwrap_or(2);
        }

        ranges_stack
    }

    /// 获取最近一层的 ranges（用于当前节点的地址转换）
    pub fn current_ranges(&self) -> Option<Vec<RangesEntry>> {
        // 需要父节点来获取 ranges
        if self.parents.is_empty() {
            return None;
        }

        let parent = self.parents.last()?;

        // 获取父节点的父节点的 address_cells
        let grandparent_address_cells = if self.parents.len() >= 2 {
            self.parents[self.parents.len() - 2]
                .address_cells()
                .unwrap_or(2)
        } else {
            2 // 根节点默认
        };

        parse_ranges_for_node(parent, grandparent_address_cells)
    }

    /// 添加路径段
    pub fn path_add(&mut self, segment: &str) {
        if !self.current_path.ends_with('/') {
            self.current_path.push('/');
        }
        self.current_path.push_str(segment);
    }

    /// 压入父节点，进入子节点前调用
    pub fn push_parent(&mut self, parent: &'a Node) {
        // 更新路径
        if self.current_path.is_empty() {
            self.current_path.push('/');
        } else if !self.current_path.ends_with('/') {
            self.current_path.push('/');
        }
        self.current_path.push_str(parent.name());

        // 压入父节点栈
        self.parents.push(parent);
    }

    /// 弹出父节点，离开子节点后调用
    pub fn pop_parent(&mut self) -> Option<&'a Node> {
        let node = self.parents.pop()?;

        // 更新路径：移除最后一个路径段
        if let Some(last_slash) = self.current_path.rfind('/') {
            self.current_path.truncate(last_slash);
            if self.current_path.is_empty() {
                self.current_path.push('/');
            }
        }

        Some(node)
    }

    /// 为进入指定子节点创建新的上下文
    /// 当前节点成为新上下文的父节点
    pub fn for_child(&self, current_node: &'a Node) -> Self {
        let mut child_ctx = Self {
            parents: self.parents.clone(),
            current_path: self.current_path.clone(),
        };
        child_ctx.push_parent(current_node);
        child_ctx
    }
}

/// 解析节点的 ranges 属性
fn parse_ranges_for_node(node: &Node, parent_address_cells: u8) -> Option<Vec<RangesEntry>> {
    let prop = node.find_property("ranges")?;
    let PropertyKind::Raw(raw) = &prop.kind else {
        return None;
    };

    // 空 ranges 表示 1:1 映射，不需要转换
    if raw.is_empty() {
        return None;
    }

    // 当前节点的 #address-cells 用于子节点地址
    let child_address_cells = node.address_cells().unwrap_or(2) as usize;
    // 父节点的 #address-cells 用于父总线地址
    let parent_addr_cells = parent_address_cells as usize;
    // 当前节点的 #size-cells
    let size_cells = node.size_cells().unwrap_or(1) as usize;

    let tuple_cells = child_address_cells + parent_addr_cells + size_cells;
    if tuple_cells == 0 {
        return None;
    }

    let words = raw.as_u32_vec();
    if words.len() % tuple_cells != 0 {
        return None;
    }

    let mut entries = Vec::with_capacity(words.len() / tuple_cells);

    for chunk in words.chunks_exact(tuple_cells) {
        let mut idx = 0;

        // 读取 child bus address
        let mut child_bus = 0u64;
        for _ in 0..child_address_cells {
            child_bus = (child_bus << 32) | chunk[idx] as u64;
            idx += 1;
        }

        // 读取 parent bus address
        let mut parent_bus = 0u64;
        for _ in 0..parent_addr_cells {
            parent_bus = (parent_bus << 32) | chunk[idx] as u64;
            idx += 1;
        }

        // 读取 length
        let mut length = 0u64;
        for _ in 0..size_cells {
            length = (length << 32) | chunk[idx] as u64;
            idx += 1;
        }

        entries.push(RangesEntry::new(child_bus, parent_bus, length));
    }

    Some(entries)
}
