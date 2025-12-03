use core::fmt;
use core::ops::Deref;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use fdt_raw::{FdtError, Phandle, Token, FDT_MAGIC};

use crate::Node;

/// Memory reservation block entry
#[derive(Clone, Debug, Default)]
pub struct MemoryReservation {
    pub address: u64,
    pub size: u64,
}

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
            root: Node::root(),
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
            memory_reservations: Vec::new(),
            root: Node::root(),
            phandle_cache: BTreeMap::new(),
        };

        // 解析内存保留块
        let data = raw_fdt.as_slice();
        let mut offset = header.off_mem_rsvmap as usize;
        loop {
            if offset + 16 > data.len() {
                break;
            }
            let address = u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap());
            let size = u64::from_be_bytes(data[offset + 8..offset + 16].try_into().unwrap());
            if address == 0 && size == 0 {
                break;
            }
            fdt.memory_reservations
                .push(MemoryReservation { address, size });
            offset += 16;
        }

        // 构建节点树
        // 使用栈来跟踪父节点，栈底是一个虚拟父节点
        let mut node_stack: Vec<Node> = Vec::new();

        for raw_node in raw_fdt.all_nodes() {
            let level = raw_node.level();
            let node = Node::from(raw_node);

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
        for child in node.children.iter() {
            let child_name = &child.name;
            let child_path = if current_path == "/" {
                format!("/{}", child_name)
            } else {
                format!("{}/{}", current_path, child_name)
            };
            self.build_phandle_cache_recursive(child, &child_path);
        }
    }

    /// 根据路径查找节点（非精确匹配）
    ///
    /// 路径格式: "/node1/node2/node3"
    /// 支持 node-name@unit-address 格式，如 "/soc/i2c@40002000"
    /// 支持 alias：如果路径不以 '/' 开头，会从 /aliases 节点解析别名
    /// 支持智能匹配：中间级别只支持精确匹配，最后一级支持前缀匹配（忽略 @unit-address）
    /// 返回所有匹配的节点及其完整路径
    pub fn find_by_path(&self, path: &str) -> Vec<(&Node, String)> {
        // 如果路径以 '/' 开头，直接按路径查找
        // 否则解析 alias
        let resolved_path = if path.starts_with('/') {
            path.to_string()
        } else {
            let Some(path) = self.resolve_alias(path) else {
                return Vec::new();
            };
            path
        };

        // 对于根路径，直接返回根节点
        let path_str = resolved_path.trim_start_matches('/');
        if path_str.is_empty() {
            return vec![(&self.root, String::from("/"))];
        }

        // 使用 Node 的智能查找功能
        self.root.find_all(&resolved_path)
    }

    /// 根据路径精确查找节点
    ///
    /// 路径格式: "/node1/node2/node3"
    /// 支持 node-name@unit-address 格式，如 "/soc/i2c@40002000"
    /// 支持 alias：如果路径不以 '/' 开头，会从 /aliases 节点解析别名
    /// 只支持精确匹配，不支持模糊匹配
    /// 返回 (节点引用, 完整路径)
    pub fn get_by_path(&self, path: &str) -> Option<(&Node, String)> {
        // 如果路径以 '/' 开头，直接按路径查找
        // 否则解析 alias
        let resolved_path = if path.starts_with('/') {
            path.to_string()
        } else {
            self.resolve_alias(path)?
        };

        // 对于根路径，直接返回根节点
        let path_str = resolved_path.trim_start_matches('/');
        if path_str.is_empty() {
            return Some((&self.root, String::from("/")));
        }

        // 使用 Node 的精确查找功能
        if let Some(node) = self.root.get_by_path(&resolved_path) {
            Some((node, resolved_path))
        } else {
            None
        }
    }

    /// 根据路径精确查找节点（可变）
    ///
    /// 支持 node-name@unit-address 格式，如 "/soc/i2c@40002000"
    /// 支持 alias：如果路径不以 '/' 开头，会从 /aliases 节点解析别名
    /// 只支持精确匹配，不支持模糊匹配
    /// 返回 (节点可变引用, 完整路径)
    pub fn get_by_path_mut(&mut self, path: &str) -> Option<(&mut Node, String)> {
        // 如果路径以 '/' 开头，直接按路径查找
        // 否则解析 alias
        let resolved_path = if path.starts_with('/') {
            path.to_string()
        } else {
            self.resolve_alias(path)?
        };

        // 对于根路径，直接返回根节点
        let path_str = resolved_path.trim_start_matches('/');
        if path_str.is_empty() {
            return Some((&mut self.root, String::from("/")));
        }

        // 使用 Node 的精确查找功能
        if let Some(node) = self.root.get_by_path_mut(&resolved_path) {
            Some((node, resolved_path))
        } else {
            None
        }
    }

    /// 根据节点名称查找所有匹配的节点
    /// 支持智能匹配（精确匹配和前缀匹配）
    pub fn find_by_name(&self, name: &str) -> Vec<(&Node, String)> {
        self.root.find_all(name)
    }

    /// 根据节点名称前缀查找所有匹配的节点
    pub fn find_by_name_prefix(&self, prefix: &str) -> Vec<(&Node, String)> {
        self.root.find_all(prefix)
    }

    /// 根据路径查找所有匹配的节点
    pub fn find_all_by_path(&self, path: &str) -> Vec<(&Node, String)> {
        // 如果路径以 '/' 开头，直接按路径查找
        // 否则解析 alias
        let resolved_path = if path.starts_with('/') {
            path.to_string()
        } else {
            let Some(path) = self.resolve_alias(path) else {
                return Vec::new();
            };
            path
        };

        // 对于根路径，直接返回根节点
        let path_str = resolved_path.trim_start_matches('/');
        if path_str.is_empty() {
            return vec![(&self.root, String::from("/"))];
        }

        // 使用 Node 的智能查找功能
        self.root.find_all(&resolved_path)
    }

    /// 解析别名，返回对应的完整路径
    ///
    /// 从 /aliases 节点查找别名对应的路径
    pub fn resolve_alias(&self, alias: &str) -> Option<String> {
        let aliases_node = self.root.get_by_path("aliases")?;
        let prop = aliases_node.find_property(alias)?;

        // 从属性中获取字符串值（路径）
        match prop {
            crate::Property::Raw(raw) => {
                // 字符串属性以 null 结尾
                let data = raw.data();
                let len = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                core::str::from_utf8(&data[..len]).ok().map(String::from)
            }
            _ => None,
        }
    }

    /// 获取所有别名
    ///
    /// 返回 (别名, 路径) 的列表
    pub fn aliases(&self) -> Vec<(&str, String)> {
        let mut result = Vec::new();
        if let Some(aliases_node) = self.root.get_by_path("aliases") {
            for prop in &aliases_node.properties {
                if let crate::Property::Raw(raw) = prop {
                    let data = raw.data();
                    let len = data.iter().position(|&b| b == 0).unwrap_or(data.len());
                    if let Ok(path) = core::str::from_utf8(&data[..len]) {
                        result.push((raw.name(), path.to_string()));
                    }
                }
            }
        }
        result
    }

    /// 根据 phandle 查找节点
    /// 返回 (节点引用, 完整路径)
    pub fn find_by_phandle(&self, phandle: Phandle) -> Option<(&Node, String)> {
        let path = self.phandle_cache.get(&phandle)?.clone();
        let node = self.root.get_by_path(&path)?;
        Some((node, path))
    }

    /// 根据 phandle 查找节点（可变）
    /// 返回 (节点可变引用, 完整路径)
    pub fn find_by_phandle_mut(&mut self, phandle: Phandle) -> Option<(&mut Node, String)> {
        let path = self.phandle_cache.get(&phandle)?.clone();
        let node = self.root.get_by_path_mut(&path)?;
        Some((node, path))
    }

    /// 获取根节点
    pub fn root(&self) -> &Node {
        &self.root
    }

    /// 获取根节点（可变）
    pub fn root_mut(&mut self) -> &mut Node {
        &mut self.root
    }

    /// 删除别名条目
    ///
    /// 从 /aliases 节点中删除指定的别名属性
    fn remove_alias_entry(&mut self, alias_name: &str) -> Result<(), FdtError> {
        if let Some(aliases_node) = self.root.get_by_path_mut("aliases") {
            // 查找并删除别名属性
            aliases_node.properties.retain(|prop| {
                if let crate::Property::Raw(raw) = prop {
                    // 检查属性名是否匹配
                    raw.name() != alias_name
                } else {
                    true
                }
            });

            // 如果 aliases 节点没有其他属性了，可以考虑删除整个节点
            // 但这里我们保留空节点以符合设备树规范
        }

        // 不论如何都返回成功，因为别名条目删除是可选的优化
        Ok(())
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
        for child in overlay.root.children.iter() {
            if child.name.starts_with("fragment@") || child.name == "fragment" {
                // fragment 格式
                self.apply_fragment(child)?;
            } else if child.name == "__overlay__" {
                // 简单格式：直接应用到根节点
                self.merge_overlay_to_root(child)?;
            } else if child.name == "__symbols__"
                || child.name == "__fixups__"
                || child.name == "__local_fixups__"
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
            .get_by_path("__overlay__")
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
        if let Some(crate::Property::Raw(raw)) = fragment.find_property("target-path") {
            let data = raw.data();
            let len = data.iter().position(|&b| b == 0).unwrap_or(data.len());
            if let Ok(path) = core::str::from_utf8(&data[..len]) {
                return Ok(path.to_string());
            }
        }

        // 使用 target（phandle 引用）
        if let Some(crate::Property::Raw(raw)) = fragment.find_property("target") {
            let data = raw.data();
            if data.len() >= 4 {
                let phandle_val = u32::from_be_bytes(data[..4].try_into().unwrap());
                let phandle = Phandle::from(phandle_val);
                // 通过 phandle 找到节点，然后构建路径
                if let Some((_node, path)) = self.find_by_phandle(phandle) {
                    return Ok(path);
                }
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
        let (target, _path) = self
            .get_by_path_mut(target_path)
            .ok_or(FdtError::NotFound)?;

        // 合并 overlay 的属性和子节点
        Self::merge_nodes(target, overlay_node);

        Ok(())
    }

    /// 合并 overlay 节点到根节点
    fn merge_overlay_to_root(&mut self, overlay: &Node) -> Result<(), FdtError> {
        // 合并属性和子节点到根节点
        for prop in &overlay.properties {
            self.root.set_property(prop.clone());
        }

        for child in &overlay.children {
            let child_name = &child.name;
            if let Some(existing) = self.root.find_child_mut(child_name) {
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
        for prop in &source.properties {
            target.set_property(prop.clone());
        }

        // 合并子节点
        for source_child in &source.children {
            let child_name = &source_child.name;
            if let Some(target_child) = target.find_child_mut(child_name) {
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
        for child in &node.children {
            if matches!(
                child.find_property("status"),
                Some(crate::Property::Status(crate::Status::Disabled))
            ) {
                to_remove.push(child.name.clone());
            }
        }

        for child_name in to_remove {
            node.remove_child(&child_name);
        }

        // 递归处理剩余子节点
        for child in node.children.iter_mut() {
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
    /// # use fdt_edit::Fdt;
    /// let mut fdt = Fdt::new();
    ///
    /// // 先添加节点再删除
    /// let mut soc = fdt_edit::Node::new("soc");
    /// let mut gpio = fdt_edit::Node::new("gpio@1000");
    /// soc.add_child(gpio);
    /// fdt.root.add_child(soc);
    ///
    /// // 精确删除节点
    /// let removed = fdt.remove_node("soc/gpio@1000")?;
    ///
    /// // 删除不存在的节点会返回 NotFound 错误
    /// // let removed = fdt.remove_node("nonexistent")?; // 这会返回 Err(NotFound)
    /// # Ok::<(), fdt_raw::FdtError>(())
    /// ```
    pub fn remove_node(&mut self, path: &str) -> Result<Option<Node>, FdtError> {
        let normalized_path = path.trim_start_matches('/');
        if normalized_path.is_empty() {
            return Err(FdtError::InvalidInput);
        }

        // 首先检查是否是别名
        if !path.starts_with('/') {
            // 可能是别名，尝试解析
            if let Some(resolved_path) = self.resolve_alias(path) {
                // 删除别名条目
                let _ = self.remove_alias_entry(path);
                // 删除实际节点
                return self.root.remove_by_path(&resolved_path);
            }
        }

        // 直接使用精确路径删除
        let result = self.root.remove_by_path(path)?;

        // 如果删除成功且结果是 None，说明路径不存在
        if result.is_none() {
            return Err(FdtError::NotFound);
        }

        Ok(result)
    }

    /// 获取所有节点的递归列表
    ///
    /// 返回包含根节点及其所有子节点的向量，按照深度优先遍历顺序
    pub fn all_nodes(&self) -> Vec<&Node> {
        let mut nodes = Vec::new();
        self.collect_all_nodes(&self.root, &mut nodes);
        nodes
    }

    /// 递归收集所有节点到结果向量中
    ///
    /// # 参数
    /// - `node`: 当前要处理的节点
    /// - `nodes`: 用于收集节点的结果向量
    fn collect_all_nodes<'a>(&'a self, node: &'a Node, nodes: &mut Vec<&'a Node>) {
        // 添加当前节点
        nodes.push(node);

        // 递归处理所有子节点
        for child in node.children.iter() {
            self.collect_all_nodes(child, nodes);
        }
    }

    /// 序列化为 FDT 二进制数据
    pub fn to_bytes(&self) -> FdtData {
        let mut builder = FdtBuilder::new();

        // 收集所有字符串
        builder.collect_strings(&self.root);

        // 构建结构块
        builder.build_struct(&self.root);

        // 生成最终数据
        builder.finalize(self.boot_cpuid_phys, &self.memory_reservations)
    }
}

/// 节点上下文信息
#[derive(Debug, Clone)]
struct NodeContext {
    /// 当前节点的 address cells
    address_cells: u8,
    /// 当前节点的 size cells
    size_cells: u8,
}

impl Default for NodeContext {
    fn default() -> Self {
        Self {
            address_cells: 1, // 默认值
            size_cells: 1,    // 默认值
        }
    }
}

/// FDT 构建器
struct FdtBuilder {
    /// 结构块数据
    struct_data: Vec<u32>,
    /// 字符串块数据
    strings_data: Vec<u8>,
    /// 字符串偏移映射
    string_offsets: Vec<(String, u32)>,
    /// 节点栈，用于跟踪当前构建的节点层次结构
    /// 栈顶是当前正在处理的节点的上下文
    node_stack: Vec<NodeContext>,
}

impl FdtBuilder {
    fn new() -> Self {
        Self {
            struct_data: Vec::new(),
            strings_data: Vec::new(),
            string_offsets: Vec::new(),
            node_stack: vec![NodeContext::default()], // 初始化栈，包含根节点上下文
        }
    }

    /// 获取当前节点的上下文
    fn current_context(&self) -> &NodeContext {
        self.node_stack.last().unwrap()
    }

    /// 推入新的节点上下文
    fn push_context(&mut self, address_cells: u8, size_cells: u8) {
        let context = NodeContext {
            address_cells,
            size_cells,
        };
        self.node_stack.push(context);
    }

    /// 弹出当前节点上下文
    fn pop_context(&mut self) {
        if self.node_stack.len() > 1 {
            // 保留根节点上下文
            self.node_stack.pop();
        }
    }

    /// 获取父节点的 address cells 和 size cells
    fn get_parent_cells(&self) -> (u8, u8) {
        if self.node_stack.len() >= 2 {
            let parent_context = &self.node_stack[self.node_stack.len() - 2];
            (parent_context.address_cells, parent_context.size_cells)
        } else {
            // 根节点的父节点，使用默认值
            (1, 1)
        }
    }

    /// 获取或添加字符串，返回偏移量
    fn get_or_add_string(&mut self, s: &str) -> u32 {
        // 查找已存在的字符串
        for (existing, offset) in &self.string_offsets {
            if existing == s {
                return *offset;
            }
        }

        // 添加新字符串
        let offset = self.strings_data.len() as u32;
        self.strings_data.extend_from_slice(s.as_bytes());
        self.strings_data.push(0); // null terminator
        self.string_offsets.push((s.into(), offset));
        offset
    }

    /// 递归收集所有属性名字符串
    fn collect_strings(&mut self, node: &Node) {
        for prop in &node.properties {
            self.get_or_add_string(prop.name());
        }
        for child in node.children.iter() {
            self.collect_strings(child);
        }
    }

    /// 构建结构块
    fn build_struct(&mut self, node: &Node) {
        self.build_node(node);
        // 添加 END token
        let token: u32 = Token::End.into();
        self.struct_data.push(token.to_be());
    }

    /// 递归构建节点
    fn build_node(&mut self, node: &Node) {
        // 首先分析当前节点的 address-cells 和 size-cells 属性
        let mut address_cells = self.current_context().address_cells;
        let mut size_cells = self.current_context().size_cells;

        // 查找节点的 address-cells 和 size-cells 属性
        for prop in &node.properties {
            match prop {
                crate::Property::AddressCells(value) => address_cells = *value,
                crate::Property::SizeCells(value) => size_cells = *value,
                _ => {}
            }
        }

        // 推入当前节点的上下文
        self.push_context(address_cells, size_cells);

        // BEGIN_NODE
        let begin_token: u32 = Token::BeginNode.into();
        self.struct_data.push(begin_token.to_be());

        // 节点名（包含 null 终止符，对齐到 4 字节）
        // 节点名是字节流，不需要进行大端转换
        let name_bytes = node.name.as_bytes();
        let name_len = name_bytes.len() + 1; // +1 for null
        let aligned_len = (name_len + 3) & !3;

        let mut name_buf = vec![0u8; aligned_len];
        name_buf[..name_bytes.len()].copy_from_slice(name_bytes);
        // null 终止符已经被 vec![0u8; ...] 填充

        // 转换为 u32 数组（保持字节顺序不变）
        for chunk in name_buf.chunks(4) {
            let word = u32::from_ne_bytes(chunk.try_into().unwrap());
            self.struct_data.push(word);
        }

        // 属性
        for prop in &node.properties {
            self.build_property(node, prop);
        }

        // 子节点
        for child in node.children.iter() {
            self.build_node(child);
        }

        // END_NODE
        let end_token: u32 = Token::EndNode.into();
        self.struct_data.push(end_token.to_be());

        // 弹出当前节点的上下文
        self.pop_context();
    }

    /// 构建属性
    fn build_property(&mut self, node: &Node, prop: &crate::Property) {
        // PROP token
        let prop_token: u32 = Token::Prop.into();
        self.struct_data.push(prop_token.to_be());

        // 获取父节点的 address_cells 和 size_cells
        let (parent_address_cells, parent_size_cells) = self.get_parent_cells();

        // 获取序列化数据（传入父节点的 cells 信息）
        let data = prop.to_bytes_with_cells(node, parent_address_cells, parent_size_cells);

        // 属性长度
        self.struct_data.push((data.len() as u32).to_be());

        // 字符串偏移
        let nameoff = self.get_or_add_string(prop.name());
        self.struct_data.push(nameoff.to_be());

        // 属性数据（对齐到 4 字节）
        // 属性数据是原始字节流，不需要大端转换
        if !data.is_empty() {
            let aligned_len = (data.len() + 3) & !3;
            let mut data_buf = vec![0u8; aligned_len];
            data_buf[..data.len()].copy_from_slice(&data);

            // 转换为 u32 数组（保持字节顺序不变）
            for chunk in data_buf.chunks(4) {
                let word = u32::from_ne_bytes(chunk.try_into().unwrap());
                self.struct_data.push(word);
            }
        }
    }

    /// 生成最终 FDT 数据
    fn finalize(self, boot_cpuid_phys: u32, memory_reservations: &[MemoryReservation]) -> FdtData {
        // 计算各部分大小和偏移
        let header_size = 40u32; // 10 * 4 bytes
        let mem_rsv_size = ((memory_reservations.len() + 1) * 16) as u32; // +1 for terminator
        let struct_size = (self.struct_data.len() * 4) as u32;
        let strings_size = self.strings_data.len() as u32;

        let off_mem_rsvmap = header_size;
        let off_dt_struct = off_mem_rsvmap + mem_rsv_size;
        let off_dt_strings = off_dt_struct + struct_size;
        let totalsize = off_dt_strings + strings_size;

        // 对齐到 4 字节
        let totalsize_aligned = (totalsize + 3) & !3;

        let mut data = Vec::with_capacity(totalsize_aligned as usize / 4);

        // Header
        data.push(FDT_MAGIC.to_be());
        data.push(totalsize_aligned.to_be());
        data.push(off_dt_struct.to_be());
        data.push(off_dt_strings.to_be());
        data.push(off_mem_rsvmap.to_be());
        data.push(17u32.to_be()); // version
        data.push(16u32.to_be()); // last_comp_version
        data.push(boot_cpuid_phys.to_be());
        data.push(strings_size.to_be());
        data.push(struct_size.to_be());

        // Memory reservation block
        for rsv in memory_reservations {
            let addr_hi = (rsv.address >> 32) as u32;
            let addr_lo = rsv.address as u32;
            let size_hi = (rsv.size >> 32) as u32;
            let size_lo = rsv.size as u32;
            data.push(addr_hi.to_be());
            data.push(addr_lo.to_be());
            data.push(size_hi.to_be());
            data.push(size_lo.to_be());
        }
        // Terminator
        data.push(0);
        data.push(0);
        data.push(0);
        data.push(0);

        // Struct block
        data.extend_from_slice(&self.struct_data);

        // Strings block（按字节复制，对齐到 4 字节）
        // 字符串数据是原始字节流，不需要大端转换
        let strings_aligned_len = (self.strings_data.len() + 3) & !3;
        let mut strings_buf = vec![0u8; strings_aligned_len];
        strings_buf[..self.strings_data.len()].copy_from_slice(&self.strings_data);

        // 转换为 u32 数组（保持字节顺序不变）
        for chunk in strings_buf.chunks(4) {
            let word = u32::from_ne_bytes(chunk.try_into().unwrap());
            data.push(word);
        }

        FdtData(data)
    }
}

/// FDT 二进制数据
#[derive(Clone, Debug)]
pub struct FdtData(Vec<u32>);

impl FdtData {
    /// 获取数据长度（字节）
    pub fn len(&self) -> usize {
        self.0.len() * 4
    }

    /// 数据是否为空
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for FdtData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe {
            core::slice::from_raw_parts(
                self.0.as_ptr() as *const u8,
                self.0.len() * core::mem::size_of::<u32>(),
            )
        }
    }
}

impl AsRef<[u8]> for FdtData {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl fmt::Display for Fdt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 写入 DTS 文件头注释
        writeln!(f, "// Device Tree Source")?;
        writeln!(f, "// Generated by fdt-edit")?;
        writeln!(f)?;

        // 写入内存保留块
        if !self.memory_reservations.is_empty() {
            writeln!(f, "/dts-v1/;")?;
            writeln!(f, "/memreserve/ ")?;
            for rsv in &self.memory_reservations {
                writeln!(f, "\t{:#x} {:#x};", rsv.address, rsv.size)?;
            }
            writeln!(f)?;
        }

        // 写入根节点
        self.fmt_node(f, &self.root, 0)?;

        Ok(())
    }
}

impl Fdt {
    /// 格式化节点为 DTS 格式
    fn fmt_node(&self, f: &mut fmt::Formatter<'_>, node: &Node, indent: usize) -> fmt::Result {
        let indent_str = "    ".repeat(indent);

        // 写入节点名（根节点名为空，显示为 /）
        let node_name = if node.name.is_empty() {
            "/"
        } else {
            &node.name
        };

        write!(f, "{}{} {{", indent_str, node_name)?;

        // 如果没有属性和子节点，写成单行
        if node.properties.is_empty() && node.children.is_empty() {
            return writeln!(f, " }};");
        }

        writeln!(f)?;

        // 写入属性
        for prop in &node.properties {
            self.fmt_property(f, prop, indent + 1)?;
        }

        // 如果有子节点，添加空行分隔
        if !node.properties.is_empty() && !node.children.is_empty() {
            writeln!(f)?;
        }

        // 写入子节点
        for child in &node.children {
            self.fmt_node(f, child, indent + 1)?;
        }

        // 写入节点结束
        writeln!(f, "{}}};", indent_str)?;

        Ok(())
    }

    /// 格式化属性为 DTS 格式
    fn fmt_property(
        &self,
        f: &mut fmt::Formatter<'_>,
        prop: &crate::Property,
        indent: usize,
    ) -> fmt::Result {
        let indent_str = "    ".repeat(indent);
        write!(f, "{}{};", indent_str, self.format_property_value(prop))?;
        writeln!(f)
    }

    /// 格式化属性值为 DTS 格式
    fn format_property_value(&self, prop: &crate::Property) -> String {
        match prop {
            crate::Property::AddressCells(value) => format!("#address-cells = <{}>", value),
            crate::Property::SizeCells(value) => format!("#size-cells = <{}>", value),
            crate::Property::InterruptCells(value) => format!("#interrupt-cells = <{}>", value),
            crate::Property::Reg(entries) => {
                let values: Vec<String> = entries
                    .iter()
                    .map(|entry| {
                        let mut result = format!("{:#x}", entry.address);
                        if let Some(size) = entry.size {
                            result.push_str(&format!(" {:#x}", size));
                        }
                        result
                    })
                    .collect();
                format!("reg = <{}>", values.join(" "))
            }
            crate::Property::Ranges {
                entries,
                child_address_cells: _,
                parent_address_cells: _,
                size_cells: _,
            } => {
                if entries.is_empty() {
                    "ranges;".to_string()
                } else {
                    let values: Vec<String> = entries
                        .iter()
                        .map(|entry| {
                            format!(
                                "{:#x} {:#x} {:#x}",
                                entry.child_bus_address, entry.parent_bus_address, entry.length
                            )
                        })
                        .collect();
                    format!("ranges = <{}>", values.join(" "))
                }
            }
            crate::Property::Compatible(values) => {
                let quoted: Vec<String> = values.iter().map(|v| format!("\"{}\"", v)).collect();
                format!("compatible = {}", quoted.join(", "))
            }
            crate::Property::Model(value) => format!("model = \"{}\"", value),
            crate::Property::Status(status) => format!(
                "status = \"{}\"",
                match status {
                    crate::Status::Okay => "okay",
                    crate::Status::Disabled => "disabled",
                }
            ),
            crate::Property::Phandle(phandle) => format!("phandle = <{:#x}>", phandle.as_usize()),
            crate::Property::LinuxPhandle(phandle) => {
                format!("linux,phandle = <{:#x}>", phandle.as_usize())
            }
            crate::Property::DeviceType(value) => format!("device_type = \"{}\"", value),
            crate::Property::InterruptParent(phandle) => {
                format!("interrupt-parent = <{:#x}>", phandle.as_usize())
            }
            crate::Property::ClockNames(values) => {
                let quoted: Vec<String> = values.iter().map(|v| format!("\"{}\"", v)).collect();
                format!("clock-names = {}", quoted.join(", "))
            }
            crate::Property::DmaCoherent => "dma-coherent;".to_string(),
            crate::Property::Raw(raw) => {
                if raw.is_empty() {
                    format!("{};", raw.name())
                } else {
                    // 尝试解析为字符串
                    if let Ok(s) = core::str::from_utf8(raw.data()) {
                        if let Some(s) = s.strip_suffix('\0') {
                            return format!("{} = \"{}\"", raw.name(), s);
                        } else {
                            return format!("{} = \"{}\"", raw.name(), s);
                        }
                    }

                    // 如果数据以 null 结尾且只包含可打印字符，当作字符串
                    if let Some(null_pos) = raw.data().iter().position(|&b| b == 0) {
                        let data_str = &raw.data()[..null_pos];
                        if data_str
                            .iter()
                            .all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace())
                        {
                            if let Ok(s) = core::str::from_utf8(data_str) {
                                return format!("{} = \"{}\"", raw.name(), s);
                            }
                        }
                    }

                    // 如果全是 4 字节对齐的数据，当作数字数组
                    if raw.data().len() % 4 == 0 {
                        let values: Vec<String> = raw
                            .data()
                            .chunks(4)
                            .map(|chunk| {
                                let array: [u8; 4] = chunk.try_into().unwrap();
                                format!("{:#x}", u32::from_be_bytes(array))
                            })
                            .collect();
                        format!("{} = <{}>", raw.name(), values.join(" "))
                    } else {
                        // 当作字节数组
                        let values: Vec<String> =
                            raw.data().iter().map(|&b| format!("{:02x}", b)).collect();
                        format!("{} = [{}]", raw.name(), values.join(" "))
                    }
                }
            }
        }
    }
}
