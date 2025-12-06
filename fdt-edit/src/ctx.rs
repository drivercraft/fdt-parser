use alloc::{collections::BTreeMap, string::String, vec::Vec};
use fdt_raw::{Phandle, Status};

use crate::{Node, NodeMut, NodeOp, NodeRef, RangesEntry, prop::PropertyKind};

// ============================================================================
// 路径遍历基础设施
// ============================================================================

/// 路径段迭代器，封装路径解析的公共逻辑
struct PathSegments<'p> {
    segments: core::iter::Peekable<core::str::Split<'p, char>>,
}

impl<'p> PathSegments<'p> {
    /// 从路径创建段迭代器（自动去除开头的 /）
    fn new(path: &'p str) -> Self {
        Self {
            segments: path.trim_start_matches('/').split('/').peekable(),
        }
    }

    /// 获取下一个非空段，返回 (段, 是否为最后一段)
    fn next_non_empty(&mut self) -> Option<(&'p str, bool)> {
        loop {
            let part = self.segments.next()?;
            if !part.is_empty() {
                let is_last = self.segments.peek().is_none();
                return Some((part, is_last));
            }
        }
    }

    /// 消费所有剩余段（用于精确遍历）
    fn for_each_non_empty<F>(&mut self, mut f: F) -> bool
    where
        F: FnMut(&'p str) -> bool,
    {
        for part in self.segments.by_ref() {
            if part.is_empty() {
                continue;
            }
            if !f(part) {
                return false;
            }
        }
        true
    }
}

/// 路径遍历器：统一 find_by_path、get_by_path 等方法的核心迭代逻辑
///
/// 通过路径逐级遍历节点树，维护 FdtContext 上下文
pub(crate) struct PathTraverser<'a, 'p> {
    /// 当前所在节点
    current: &'a Node,
    /// 路径段迭代器
    segments: PathSegments<'p>,
    /// 遍历上下文
    ctx: FdtContext<'a>,
}

impl<'a, 'p> PathTraverser<'a, 'p> {
    /// 创建新的路径遍历器
    ///
    /// # 参数
    /// - `root`: 根节点
    /// - `path`: 已规范化的路径（以 `/` 开头，不含别名）
    /// - `ctx`: 初始上下文（可包含 phandle_map）
    pub(crate) fn new(root: &'a Node, path: &'p str, ctx: FdtContext<'a>) -> Self {
        Self {
            current: root,
            segments: PathSegments::new(path),
            ctx,
        }
    }

    /// 精确遍历到目标节点（用于 get_by_path）
    /// 返回 None 表示路径不存在
    pub(crate) fn traverse_exact(mut self) -> Option<NodeRef<'a>> {
        let success = self.segments.for_each_non_empty(|part| {
            self.ctx.push_parent(self.current);
            if let Some(child) = self.current.find_child_exact(part) {
                self.current = child;
                true
            } else {
                false
            }
        });

        if success {
            self.ctx.path_add(self.current.name());
            Some(NodeRef::new(self.current, self.ctx))
        } else {
            None
        }
    }

    /// 模糊遍历（用于 find_by_path）
    /// 中间段精确匹配，最后一段模糊匹配，返回所有匹配的节点
    pub(crate) fn traverse_fuzzy(mut self) -> Vec<NodeRef<'a>> {
        let mut results = Vec::new();

        while let Some((part, is_last)) = self.segments.next_non_empty() {
            self.ctx.push_parent(self.current);

            if is_last {
                // 最后一段：模糊匹配，收集所有结果
                for child in self.current.find_child(part) {
                    let mut child_ctx = self.ctx.clone();
                    child_ctx.path_add(child.name());
                    results.push(NodeRef::new(child, child_ctx));
                }
                return results;
            } else {
                // 中间段：精确匹配
                let Some(child) = self.current.find_child_exact(part) else {
                    return results;
                };
                self.current = child;
            }
        }

        results
    }
}

/// 可变路径遍历器
pub(crate) struct PathTraverserMut<'a, 'p> {
    /// 当前所在节点
    current: &'a mut Node,
    /// 路径段列表
    segments: Vec<&'p str>,
    /// 当前段索引
    index: usize,
    /// 遍历上下文
    ctx: FdtContext<'a>,
}

impl<'a, 'p> PathTraverserMut<'a, 'p> {
    pub(crate) fn new(root: &'a mut Node, path: &'p str, ctx: FdtContext<'a>) -> Self {
        // 预处理：过滤空段
        let segments: Vec<_> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            current: root,
            segments,
            index: 0,
            ctx,
        }
    }

    /// 精确遍历到目标节点（可变版本）
    pub(crate) fn traverse_exact(mut self) -> Option<NodeMut<'a>> {
        while self.index < self.segments.len() {
            let part = self.segments[self.index];
            self.index += 1;
            self.ctx.path_add(self.current.name());
            self.current = self.current.find_child_exact_mut(part)?;
        }

        self.ctx.path_add(self.current.name());
        Some(NodeMut {
            node: self.current,
            ctx: self.ctx,
        })
    }
}

// ============================================================================
// FDT 上下文
// ============================================================================

/// 遍历上下文，存储从根到当前节点的父节点引用栈
#[derive(Clone, Debug, Default)]
pub struct FdtContext<'a> {
    /// 父节点引用栈（从根节点到当前节点的父节点）
    /// 栈底是根节点，栈顶是当前节点的直接父节点
    pub parents: Vec<&'a Node>,
    /// 当前节点的完整路径
    pub current_path: String,
    /// phandle 到节点引用的映射
    /// 用于通过 phandle 快速查找节点（如中断父节点）
    pub phandle_map: BTreeMap<Phandle, &'a Node>,
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
        if !self.current_path.ends_with('/') {
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
            phandle_map: self.phandle_map.clone(),
        };
        child_ctx.push_parent(current_node);
        child_ctx
    }

    /// 通过 phandle 查找节点
    pub fn find_by_phandle(&self, phandle: Phandle) -> Option<&'a Node> {
        self.phandle_map.get(&phandle).copied()
    }

    /// 设置 phandle 到节点的映射
    pub fn set_phandle_map(&mut self, map: BTreeMap<Phandle, &'a Node>) {
        self.phandle_map = map;
    }

    /// 从 Fdt 构建 phandle 映射
    pub fn build_phandle_map_from_node(node: &'a Node, map: &mut BTreeMap<Phandle, &'a Node>) {
        if let Some(phandle) = node.phandle() {
            map.insert(phandle, node);
        }
        for child in node.children() {
            Self::build_phandle_map_from_node(child, map);
        }
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
