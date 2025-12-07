use alloc::{collections::BTreeMap, string::String, vec::Vec};

use fdt_raw::{Phandle, Status};

use crate::{Node, RangesEntry};

// ============================================================================
// FDT 上下文
// ============================================================================

/// 遍历上下文，存储从根到当前节点的父节点引用栈
#[derive(Clone, Default)]
pub struct Context<'a> {
    /// 父节点引用栈（从根节点到当前节点的父节点）
    /// 栈底是根节点，栈顶是当前节点的直接父节点
    pub parents: Vec<&'a Node>,

    /// phandle 到节点引用的映射
    /// 用于通过 phandle 快速查找节点（如中断父节点）
    pub phandle_map: BTreeMap<Phandle, &'a Node>,
}

impl<'a> Context<'a> {
    /// 创建新的上下文
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_path(&self) -> String {
        self.parents
            .iter()
            .map(|n| n.name())
            .collect::<Vec<_>>()
            .join("/")
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
    pub fn parent_address_cells(&self) -> u32 {
        self.parent().and_then(|p| p.address_cells()).unwrap_or(2)
    }

    /// 获取父节点的 #size-cells
    /// 优先从直接父节点获取，否则返回默认值 1
    pub fn parent_size_cells(&self) -> u32 {
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
        let mut prev_address_cells = 2; // 根节点默认

        for parent in &self.parents {
            if let Some(ranges) = parent.ranges(prev_address_cells) {
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
        parent.ranges(grandparent_address_cells)
    }

    pub fn push(&mut self, node: &'a Node) {
        self.parents.push(node);
    }

    /// 通过 phandle 查找节点
    pub fn find_by_phandle(&self, phandle: Phandle) -> Option<&'a Node> {
        self.phandle_map.get(&phandle).copied()
    }

    /// 从 Fdt 构建 phandle 映射
    pub fn build_phandle_map_from_node(node: &'a Node, map: &mut BTreeMap<Phandle, &'a Node>) {
        if let Some(phandle) = node.phandle() {
            map.insert(phandle, node);
        }
        for child in &node.children {
            Self::build_phandle_map_from_node(child, map);
        }
    }
}
