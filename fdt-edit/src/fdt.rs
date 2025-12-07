use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

pub use fdt_raw::MemoryReservation;
use fdt_raw::{FdtError, Phandle, Status};

use crate::{Node, NodeIter, NodeIterMut, NodeMut, NodeRef};

/// 可编辑的 FDT
#[derive(Clone, Debug)]
pub struct Fdt {
    /// 引导 CPU ID
    pub boot_cpuid_phys: u32,
    /// 内存保留块
    pub memory_reservations: Vec<MemoryReservation>,
    /// 根节点
    pub root: Node,
    /// phandle 到节点完整路径的缓存
    phandle_cache: BTreeMap<Phandle, String>,
}

impl Default for Fdt {
    fn default() -> Self {
        Self::new()
    }
}

impl Fdt {
    /// 创建新的空 FDT
    pub fn new() -> Self {
        Self {
            boot_cpuid_phys: 0,
            memory_reservations: Vec::new(),
            root: Node::new(""),
            phandle_cache: BTreeMap::new(),
        }
    }

    /// 从原始 FDT 数据解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, FdtError> {
        let raw_fdt = fdt_raw::Fdt::from_bytes(data)?;
        Self::from_raw(&raw_fdt)
    }

    /// 从原始指针解析
    ///
    /// # Safety
    /// 调用者必须确保指针有效且指向有效的 FDT 数据
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Self, FdtError> {
        let raw_fdt = unsafe { fdt_raw::Fdt::from_ptr(ptr)? };
        Self::from_raw(&raw_fdt)
    }

    /// 从 fdt_raw::Fdt 转换
    fn from_raw(raw_fdt: &fdt_raw::Fdt) -> Result<Self, FdtError> {
        let header = raw_fdt.header();

        let mut fdt = Fdt {
            boot_cpuid_phys: header.boot_cpuid_phys,
            memory_reservations: raw_fdt.memory_reservations().collect(),
            root: Node::new(""),
            phandle_cache: BTreeMap::new(),
        };

        // 构建节点树
        // 使用栈来跟踪父节点，栈底是一个虚拟父节点
        let mut node_stack: Vec<Node> = Vec::new();

        for raw_node in raw_fdt.all_nodes() {
            let level = raw_node.level();
            let node = Node::from(&raw_node);

            // 弹出栈直到达到正确的父级别
            // level 0 = 根节点，应该直接放入空栈
            // level 1 = 根节点的子节点，栈中应该只有根节点
            while node_stack.len() > level {
                let child = node_stack.pop().unwrap();
                if let Some(parent) = node_stack.last_mut() {
                    parent.add_child(child);
                } else {
                    // 这是根节点
                    fdt.root = child;
                }
            }

            node_stack.push(node);
        }

        // 弹出所有剩余节点
        while let Some(child) = node_stack.pop() {
            if let Some(parent) = node_stack.last_mut() {
                parent.add_child(child);
            } else {
                // 这是根节点
                fdt.root = child;
            }
        }

        // 构建 phandle 缓存
        fdt.rebuild_phandle_cache();

        Ok(fdt)
    }

    /// 重建 phandle 缓存
    pub fn rebuild_phandle_cache(&mut self) {
        self.phandle_cache.clear();
        let root_clone = self.root.clone();
        self.build_phandle_cache_recursive(&root_clone, "/");
    }

    /// 递归构建 phandle 缓存
    fn build_phandle_cache_recursive(&mut self, node: &Node, current_path: &str) {
        // 检查节点是否有 phandle 属性
        if let Some(phandle) = node.phandle() {
            self.phandle_cache.insert(phandle, current_path.to_string());
        }

        // 递归处理子节点
        for child in &node.children {
            let child_name = child.name();
            let child_path = if current_path == "/" {
                format!("/{}", child_name)
            } else {
                format!("{}/{}", current_path, child_name)
            };
            self.build_phandle_cache_recursive(child, &child_path);
        }
    }

    pub fn find_by_path<'a>(&'a self, path: &str) -> impl Iterator<Item = NodeRef<'a>> {
        let path = self
            .normalize_path(path)
            .unwrap_or_else(|| path.to_string());

        NodeIter::new(self.root()).filter_map(move |node_ref| {
            if node_ref.path_eq_fuzzy(&path) {
                Some(node_ref)
            } else {
                None
            }
        })
    }

    pub fn get_by_path<'a>(&'a self, path: &str) -> Option<NodeRef<'a>> {
        let path = self.normalize_path(path)?;
        NodeIter::new(self.root()).find_map(move |node_ref| {
            if node_ref.path_eq(&path) {
                Some(node_ref)
            } else {
                None
            }
        })
    }

    pub fn get_by_path_mut<'a>(&'a mut self, path: &str) -> Option<NodeMut<'a>> {
        let path = self.normalize_path(path)?;
        NodeIterMut::new(self.root_mut()).find_map(move |node_mut| {
            if node_mut.path_eq(&path) {
                Some(node_mut)
            } else {
                None
            }
        })
    }

    /// 规范化路径：如果是别名则解析为完整路径，否则确保以 / 开头
    fn normalize_path(&self, path: &str) -> Option<String> {
        if path.starts_with('/') {
            Some(path.to_string())
        } else {
            // 尝试解析别名
            self.resolve_alias(path).map(|s| s.to_string())
        }
    }

    /// 解析别名，返回对应的完整路径
    ///
    /// 从 /aliases 节点查找别名对应的路径
    pub fn resolve_alias(&self, alias: &str) -> Option<&str> {
        let aliases_node = self.get_by_path("/aliases")?;
        let prop = aliases_node.find_property(alias)?;
        prop.as_str()
    }

    /// 获取所有别名
    ///
    /// 返回 (别名, 路径) 的列表
    pub fn aliases(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if let Some(aliases_node) = self.get_by_path("/aliases") {
            for prop in aliases_node.properties() {
                let name = prop.name().to_string();
                let path = prop.as_str().unwrap().to_string();
                result.push((name, path));
            }
        }
        result
    }

    /// 根据 phandle 查找节点
    /// 返回 (节点引用, 完整路径)
    pub fn find_by_phandle(&self, phandle: Phandle) -> Option<NodeRef<'_>> {
        let path = self.phandle_cache.get(&phandle)?.clone();
        self.get_by_path(&path)
    }

    /// 根据 phandle 查找节点（可变）
    /// 返回 (节点可变引用, 完整路径)
    pub fn find_by_phandle_mut(&mut self, phandle: Phandle) -> Option<NodeMut<'_>> {
        let path = self.phandle_cache.get(&phandle)?.clone();
        self.get_by_path_mut(&path)
    }

    /// 获取根节点
    pub fn root(&self) -> &Node {
        &self.root
    }

    /// 获取根节点（可变）
    pub fn root_mut(&mut self) -> &mut Node {
        &mut self.root
    }

    /// 应用设备树覆盖 (Device Tree Overlay)
    ///
    /// 支持两种 overlay 格式：
    /// 1. fragment 格式：包含 fragment@N 节点，每个 fragment 有 target/target-path 和 __overlay__
    /// 2. 简单格式：直接包含 __overlay__ 节点
    ///
    /// # 示例
    /// ```ignore
    /// // fragment 格式
    /// fragment@0 {
    ///     target-path = "/soc";
    ///     __overlay__ {
    ///         new_node { ... };
    ///     };
    /// };
    /// ```
    pub fn apply_overlay(&mut self, overlay: &Fdt) -> Result<(), FdtError> {
        // 遍历 overlay 根节点的所有子节点
        for child in &overlay.root.children {
            if child.name().starts_with("fragment@") || child.name() == "fragment" {
                // fragment 格式
                self.apply_fragment(child)?;
            } else if child.name() == "__overlay__" {
                // 简单格式：直接应用到根节点
                self.merge_overlay_to_root(child)?;
            } else if child.name() == "__symbols__"
                || child.name() == "__fixups__"
                || child.name() == "__local_fixups__"
            {
                // 跳过这些特殊节点
                continue;
            }
        }

        // 重建 phandle 缓存
        self.rebuild_phandle_cache();

        Ok(())
    }

    /// 应用单个 fragment
    fn apply_fragment(&mut self, fragment: &Node) -> Result<(), FdtError> {
        // 获取目标路径
        let target_path = self.resolve_fragment_target(fragment)?;

        // 找到 __overlay__ 子节点
        let overlay_node = fragment
            .get_child("__overlay__")
            .ok_or(FdtError::NotFound)?;

        // 找到目标节点并应用覆盖
        // 需要克隆路径因为后面要修改 self
        let target_path_owned = target_path.to_string();

        // 应用覆盖到目标节点
        self.apply_overlay_to_target(&target_path_owned, overlay_node)?;

        Ok(())
    }

    /// 解析 fragment 的目标路径
    fn resolve_fragment_target(&self, fragment: &Node) -> Result<String, FdtError> {
        // 优先使用 target-path（字符串路径）
        if let Some(prop) = fragment.get_property("target-path") {
            return Ok(prop.as_str().ok_or(FdtError::Utf8Parse)?.to_string());
        }

        // 使用 target（phandle 引用）
        if let Some(prop) = fragment.get_property("target") {
            let ph = prop.get_u32().ok_or(FdtError::InvalidInput)?;
            let ph = Phandle::from(ph);

            // 通过 phandle 找到节点，然后构建路径
            if let Some(node) = self.find_by_phandle(ph) {
                return Ok(node.path());
            }
        }

        Err(FdtError::NotFound)
    }

    /// 将 overlay 应用到目标节点
    fn apply_overlay_to_target(
        &mut self,
        target_path: &str,
        overlay_node: &Node,
    ) -> Result<(), FdtError> {
        // 找到目标节点
        let mut target = self
            .get_by_path_mut(target_path)
            .ok_or(FdtError::NotFound)?;

        // 合并 overlay 的属性和子节点
        Self::merge_nodes(target.node, overlay_node);

        Ok(())
    }

    /// 合并 overlay 节点到根节点
    fn merge_overlay_to_root(&mut self, overlay: &Node) -> Result<(), FdtError> {
        // 合并属性和子节点到根节点
        for prop in overlay.properties() {
            self.root.set_property(prop.clone());
        }

        for child in overlay.children() {
            let child_name = child.name();
            if let Some(existing) = self.root.get_child_mut(child_name) {
                // 合并到现有子节点
                Self::merge_nodes(existing, child);
            } else {
                // 添加新子节点
                self.root.add_child(child.clone());
            }
        }

        Ok(())
    }

    /// 递归合并两个节点
    fn merge_nodes(target: &mut Node, source: &Node) {
        // 合并属性（source 覆盖 target）
        for prop in source.properties() {
            target.set_property(prop.clone());
        }

        // 合并子节点
        for source_child in source.children() {
            let child_name = &source_child.name();
            if let Some(target_child) = target.get_child_mut(child_name) {
                // 递归合并
                Self::merge_nodes(target_child, source_child);
            } else {
                // 添加新子节点
                target.add_child(source_child.clone());
            }
        }
    }

    /// 删除节点（通过设置 status = "disabled" 或直接删除）
    ///
    /// 如果 overlay 中的节点有 status = "disabled"，则禁用目标节点
    pub fn apply_overlay_with_delete(
        &mut self,
        overlay: &Fdt,
        delete_disabled: bool,
    ) -> Result<(), FdtError> {
        self.apply_overlay(overlay)?;

        if delete_disabled {
            // 移除所有 status = "disabled" 的节点
            Self::remove_disabled_nodes(&mut self.root);
            self.rebuild_phandle_cache();
        }

        Ok(())
    }

    /// 递归移除 disabled 的节点
    fn remove_disabled_nodes(node: &mut Node) {
        // 移除 disabled 的子节点
        let mut to_remove = Vec::new();
        for child in node.children() {
            if matches!(child.status(), Some(Status::Disabled)) {
                to_remove.push(child.name().to_string());
            }
        }

        for child_name in to_remove {
            node.remove_child(&child_name);
        }

        // 递归处理剩余子节点
        for child in node.children_mut() {
            Self::remove_disabled_nodes(child);
        }
    }

    /// 通过精确路径删除节点及其子树
    /// 只支持精确路径匹配，不支持模糊匹配
    /// 支持通过别名删除节点，并自动删除对应的别名条目
    ///
    /// # 参数
    /// - `path`: 删除路径，格式如 "soc/gpio@1000" 或 "/soc/gpio@1000" 或别名
    ///
    /// # 返回值
    /// `Ok(Option<Node>)`: 如果找到并删除了节点，返回被删除的节点；如果路径不存在，返回 None
    /// `Err(FdtError)`: 如果路径格式无效
    ///
    /// # 示例
    /// ```rust
    /// # use fdt_edit::{Fdt, Node, NodeOp};
    /// let mut fdt = Fdt::new();
    ///
    /// // 先添加节点再删除
    /// let mut soc = Node::new_raw("soc");
    /// soc.add_child(Node::new_raw("gpio@1000"));
    /// fdt.root.add_child(soc);
    ///
    /// // 精确删除节点
    /// let removed = fdt.remove_node("soc/gpio@1000")?;
    /// assert!(removed.is_some());
    /// # Ok::<(), fdt_raw::FdtError>(())
    /// ```
    pub fn remove_node(&mut self, path: &str) -> Result<Option<Node>, FdtError> {
        let normalized_path = self.normalize_path(path).ok_or(FdtError::InvalidInput)?;

        // 直接使用精确路径删除
        let result = self.root.remove_by_path(&normalized_path)?;

        // 如果删除成功且结果是 None，说明路径不存在
        if result.is_none() {
            return Err(FdtError::NotFound);
        }

        Ok(result)
    }

    /// 获取所有节点的深度优先迭代器
    ///
    /// 返回包含根节点及其所有子节点的迭代器，按照深度优先遍历顺序
    pub fn all_nodes(&self) -> impl Iterator<Item = NodeRef<'_>> + '_ {
        NodeIter::new(&self.root)
    }

    pub fn all_nodes_mut(&mut self) -> impl Iterator<Item = NodeMut<'_>> + '_ {
        NodeIterMut::new(&mut self.root)
    }

    pub fn find_compatible(&self, compatible: &[&str]) -> Vec<NodeRef<'_>> {
        let mut results = Vec::new();
        for node_ref in self.all_nodes() {
            let Some(ls) = node_ref.compatible() else {
                continue;
            };

            for comp in ls {
                if compatible.contains(&comp) {
                    results.push(node_ref);
                    break;
                }
            }
        }
        results
    }
}
