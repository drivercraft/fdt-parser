use alloc::{string::String, string::ToString, vec::Vec};
use fdt_raw::{Phandle, Status};

use crate::{Node, NodeOp, RangesEntry};

#[derive(Clone, Debug)]
pub struct FdtContext {
    /// 父节点路径栈
    pub parents: Vec<String>,
    /// 父节点的 #address-cells
    pub parent_address_cells: u8,
    /// 父节点的 #size-cells
    pub parent_size_cells: u8,
    /// 多重父级 ranges 转换条目
    /// 每层父节点的 ranges 转换信息
    pub ranges: Vec<Vec<RangesEntry>>,
    /// 中断父节点 phandle
    pub interrupt_parent: Option<Phandle>,
    /// 当前节点的完整路径
    pub current_path: String,
    /// 递归深度
    pub depth: usize,
    /// 节点是否被禁用
    pub disabled: bool,
}

impl Default for FdtContext {
    fn default() -> Self {
        Self {
            parents: Vec::new(),
            parent_address_cells: 2, // 默认值
            parent_size_cells: 1,    // 默认值
            ranges: Vec::new(),      // 多重父级 ranges
            interrupt_parent: None,
            current_path: String::from(""),
            depth: 1,
            disabled: false,
        }
    }
}

impl FdtContext {
    /// 创建新的上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建用于根节点的上下文
    pub fn for_root() -> Self {
        Self::default()
    }

    pub fn path_add(&mut self, segment: &str) {
        if !self.current_path.ends_with("/") {
            self.current_path.push('/');
        }
        self.current_path.push_str(segment);
    }

    pub fn update_node(&mut self, node: &Node) {
        self.parent_address_cells = 2;
        self.parent_size_cells = 1;

        // 更新上下文
        if let Some(v) = node.address_cells() {
            self.parent_address_cells = v;
        }
        if let Some(v) = node.size_cells() {
            self.parent_size_cells = v;
        }

        if let Some(v) = node.interrupt_parent() {
            self.interrupt_parent = Some(v);
        }

        if matches!(node.status(), Some(Status::Disabled)) {
            self.disabled = true;
        }

        if !self.current_path.is_empty() {
            self.parents.push(self.current_path.clone());
        }
        self.path_add(node.name());
        self.depth += 1;
    }
}
