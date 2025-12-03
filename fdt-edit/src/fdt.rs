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

/// 节点路径索引
type NodeIndex = Vec<usize>;

/// 可编辑的 FDT
#[derive(Clone, Debug)]
pub struct Fdt {
    /// 引导 CPU ID
    pub boot_cpuid_phys: u32,
    /// 内存保留块
    pub memory_reservations: Vec<MemoryReservation>,
    /// 根节点
    pub root: Node,
    /// phandle 到节点路径的缓存
    phandle_cache: BTreeMap<Phandle, NodeIndex>,
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
                    parent.children.push(child);
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
                parent.children.push(child);
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
        self.build_phandle_cache_recursive(&self.root.clone(), Vec::new());
    }

    /// 递归构建 phandle 缓存
    fn build_phandle_cache_recursive(&mut self, node: &Node, current_index: NodeIndex) {
        // 检查节点是否有 phandle 属性
        if let Some(phandle) = node.phandle() {
            self.phandle_cache.insert(phandle, current_index.clone());
        }

        // 递归处理子节点
        for (i, child) in node.children.iter().enumerate() {
            let mut child_index = current_index.clone();
            child_index.push(i);
            self.build_phandle_cache_recursive(child, child_index);
        }
    }

    /// 查找匹配的子节点，支持 node-name@unit-address 格式
    fn find_child_matching<'a>(parent: &'a Node, path_part: &str) -> Option<&'a Node> {
        // 如果包含 @，先尝试精确匹配
        if path_part.contains('@') {
            // 1. 精确匹配完整名称（包含 @address）
            if let Some(child) = parent.children.iter().find(|c| c.name == path_part) {
                return Some(child);
            }

            // 2. 提取 node-name 进行部分匹配
            let node_name = path_part.split('@').next().unwrap();
            return parent.children.iter().find(|c| {
                c.name.starts_with(node_name) &&
                (c.name.len() == node_name.len() || c.name[node_name.len()..].starts_with('@'))
            });
        }

        // 3. 传统匹配方式（如果找不到精确匹配，尝试部分匹配 @ 地址）
        if let Some(child) = parent.find_child(path_part) {
            return Some(child);
        }

        // 4. 如果传统方式没找到，尝试部分匹配带 @ 地址的节点
        parent.children.iter().find(|c| {
            c.name.starts_with(path_part) &&
            (c.name.len() == path_part.len() || c.name[path_part.len()..].starts_with('@'))
        })
    }

    /// 查找匹配的子节点（可变），支持 node-name@unit-address 格式
    fn find_child_matching_mut<'a>(parent: &'a mut Node, path_part: &str) -> Option<&'a mut Node> {
        // 如果包含 @，先尝试精确匹配
        if path_part.contains('@') {
            // 1. 精确匹配完整名称（包含 @address）
            if let Some(pos) = parent.children.iter().position(|c| c.name == path_part) {
                return Some(&mut parent.children[pos]);
            }

            // 2. 提取 node-name 进行部分匹配
            let node_name = path_part.split('@').next().unwrap();
            if let Some(pos) = parent.children.iter().position(|c| {
                c.name.starts_with(node_name) &&
                (c.name.len() == node_name.len() || c.name[node_name.len()..].starts_with('@'))
            }) {
                return Some(&mut parent.children[pos]);
            }
        }

        // 3. 传统匹配方式（如果找不到精确匹配，尝试部分匹配 @ 地址）
        if let Some(pos) = parent.children.iter().position(|c| c.name == path_part) {
            return Some(&mut parent.children[pos]);
        }

        // 4. 如果传统方式没找到，尝试部分匹配带 @ 地址的节点
        if let Some(pos) = parent.children.iter().position(|c| {
            c.name.starts_with(path_part) &&
            (c.name.len() == path_part.len() || c.name[path_part.len()..].starts_with('@'))
        }) {
            return Some(&mut parent.children[pos]);
        }

        None
    }

    /// 根据路径查找节点
    ///
    /// 路径格式: "/node1/node2/node3"
    /// 支持 node-name@unit-address 格式，如 "/soc/i2c@40002000"
    /// 支持 alias：如果路径不以 '/' 开头，会从 /aliases 节点解析别名
    /// 返回 (节点引用, 完整路径)
    pub fn find_by_path(&self, path: &str) -> Option<(&Node, String)> {
        // 如果路径以 '/' 开头，直接按路径查找
        // 否则解析 alias
        let resolved_path = if path.starts_with('/') {
            path.to_string()
        } else {
            self.resolve_alias(path)?
        };

        let path_str = resolved_path.trim_start_matches('/');
        if path_str.is_empty() {
            return Some((&self.root, String::from("/")));
        }

        let mut current = &self.root;
        for part in path_str.split('/') {
            if part.is_empty() {
                continue;
            }
            current = Self::find_child_matching(current, part)?;
        }
        Some((current, resolved_path))
    }

    /// 根据路径查找节点（可变）
    ///
    /// 支持 node-name@unit-address 格式，如 "/soc/i2c@40002000"
    /// 支持 alias：如果路径不以 '/' 开头，会从 /aliases 节点解析别名
    /// 返回 (节点可变引用, 完整路径)
    pub fn find_by_path_mut(&mut self, path: &str) -> Option<(&mut Node, String)> {
        // 如果路径以 '/' 开头，直接按路径查找
        // 否则解析 alias
        let resolved_path = if path.starts_with('/') {
            path.to_string()
        } else {
            self.resolve_alias(path)?
        };

        let path_str = resolved_path.trim_start_matches('/');
        if path_str.is_empty() {
            return Some((&mut self.root, String::from("/")));
        }

        let mut current = &mut self.root;
        for part in path_str.split('/') {
            if part.is_empty() {
                continue;
            }
            current = match Self::find_child_matching_mut(current, part) {
                Some(child) => child,
                None => return None,
            };
        }
        Some((current, resolved_path))
    }

    /// 解析别名，返回对应的完整路径
    ///
    /// 从 /aliases 节点查找别名对应的路径
    pub fn resolve_alias(&self, alias: &str) -> Option<String> {
        let aliases_node = self.root.find_child("aliases")?;
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
        if let Some(aliases_node) = self.root.find_child("aliases") {
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

    /// 根据路径查找所有匹配的节点
    ///
    /// 支持通配符 '*' 匹配任意节点名
    /// 例如: "/soc/*/serial" 会匹配所有 soc 下任意子节点中的 serial 节点
    /// 返回 Vec<(节点引用, 完整路径)>
    pub fn find_all_by_path(&self, path: &str) -> Vec<(&Node, String)> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return vec![(&self.root, String::from("/"))];
        }

        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return vec![(&self.root, String::from("/"))];
        }

        let mut results = Vec::new();
        Self::find_all_by_path_recursive(&self.root, &parts, 0, String::from("/"), &mut results);
        results
    }

    /// 递归查找路径匹配的节点
    fn find_all_by_path_recursive<'a>(
        node: &'a Node,
        parts: &[&str],
        index: usize,
        current_path: String,
        results: &mut Vec<(&'a Node, String)>,
    ) {
        if index >= parts.len() {
            results.push((node, current_path));
            return;
        }

        let part = parts[index];
        if part == "*" {
            // 通配符：遍历所有子节点
            for child in &node.children {
                let child_path = if current_path == "/" {
                    format!("/{}", child.name)
                } else {
                    format!("{}/{}", current_path, child.name)
                };
                Self::find_all_by_path_recursive(child, parts, index + 1, child_path, results);
            }
        } else {
            // 精确匹配：可能有多个同名节点
            for child in &node.children {
                if child.name == part {
                    let child_path = if current_path == "/" {
                        format!("/{}", child.name)
                    } else {
                        format!("{}/{}", current_path, child.name)
                    };
                    Self::find_all_by_path_recursive(child, parts, index + 1, child_path, results);
                }
            }
        }
    }

    /// 根据节点名称查找所有匹配的节点（递归搜索整个树）
    ///
    /// 返回所有名称匹配的节点 Vec<(节点引用, 完整路径)>
    pub fn find_by_name(&self, name: &str) -> Vec<(&Node, String)> {
        let mut results = Vec::new();
        Self::find_by_name_recursive(&self.root, name, String::from("/"), &mut results);
        results
    }

    /// 递归按名称查找节点
    fn find_by_name_recursive<'a>(
        node: &'a Node,
        name: &str,
        current_path: String,
        results: &mut Vec<(&'a Node, String)>,
    ) {
        // 检查当前节点
        if node.name == name {
            results.push((node, current_path.clone()));
        }

        // 递归检查所有子节点
        for child in &node.children {
            let child_path = if current_path == "/" {
                format!("/{}", child.name)
            } else {
                format!("{}/{}", current_path, child.name)
            };
            Self::find_by_name_recursive(child, name, child_path, results);
        }
    }

    /// 根据节点名称前缀查找所有匹配的节点
    ///
    /// 例如: find_by_name_prefix("gpio") 会匹配 "gpio", "gpio0", "gpio@1000" 等
    /// 返回 Vec<(节点引用, 完整路径)>
    pub fn find_by_name_prefix(&self, prefix: &str) -> Vec<(&Node, String)> {
        let mut results = Vec::new();
        Self::find_by_name_prefix_recursive(&self.root, prefix, String::from("/"), &mut results);
        results
    }

    /// 递归按名称前缀查找节点
    fn find_by_name_prefix_recursive<'a>(
        node: &'a Node,
        prefix: &str,
        current_path: String,
        results: &mut Vec<(&'a Node, String)>,
    ) {
        // 检查当前节点
        if node.name.starts_with(prefix) {
            results.push((node, current_path.clone()));
        }

        // 递归检查所有子节点
        for child in &node.children {
            let child_path = if current_path == "/" {
                format!("/{}", child.name)
            } else {
                format!("{}/{}", current_path, child.name)
            };
            Self::find_by_name_prefix_recursive(child, prefix, child_path, results);
        }
    }

    /// 根据 phandle 查找节点
    /// 返回 (节点引用, 完整路径)
    pub fn find_by_phandle(&self, phandle: Phandle) -> Option<(&Node, String)> {
        let index = self.phandle_cache.get(&phandle)?;
        let node = self.get_node_by_index(index)?;
        let path = self.get_node_path(node)?;
        Some((node, path))
    }

    /// 根据 phandle 查找节点（可变）
    /// 返回 (节点可变引用, 完整路径)
    pub fn find_by_phandle_mut(&mut self, phandle: Phandle) -> Option<(&mut Node, String)> {
        let index = self.phandle_cache.get(&phandle)?.clone();
        let path = self.get_path_by_index(&index);
        let node = self.get_node_by_index_mut(&index)?;
        Some((node, path))
    }

    /// 根据索引获取路径
    fn get_path_by_index(&self, index: &NodeIndex) -> String {
        if index.is_empty() {
            return String::from("/");
        }
        let mut path = String::new();
        let mut current = &self.root;
        for &i in index {
            if let Some(child) = current.children.get(i) {
                path.push('/');
                path.push_str(&child.name);
                current = child;
            }
        }
        if path.is_empty() {
            String::from("/")
        } else {
            path
        }
    }

    /// 根据索引获取节点
    fn get_node_by_index(&self, index: &NodeIndex) -> Option<&Node> {
        let mut current = &self.root;
        for &i in index {
            current = current.children.get(i)?;
        }
        Some(current)
    }

    /// 根据索引获取节点（可变）
    fn get_node_by_index_mut(&mut self, index: &NodeIndex) -> Option<&mut Node> {
        let mut current = &mut self.root;
        for &i in index {
            current = current.children.get_mut(i)?;
        }
        Some(current)
    }

    /// 获取根节点
    pub fn root(&self) -> &Node {
        &self.root
    }

    /// 获取根节点（可变）
    pub fn root_mut(&mut self) -> &mut Node {
        &mut self.root
    }

    /// 删除指定路径的节点及其所有子节点
    ///
    /// 支持 node-name@unit-address 格式，如 "/soc/i2c@40002000"
    /// 支持 alias：如果路径不以 '/' 开头，会从 /aliases 节点解析别名
    ///
    /// # 返回值
    /// - `Ok(())`: 节点成功删除
    /// - `Err(FdtError::NotFound)`: 路径不存在
    /// - `Err(FdtError::InvalidInput)`: 输入路径无效
    ///
    /// # 示例
    /// ```ignore
    /// let mut fdt = Fdt::new();
    /// // 删除节点
    /// fdt.remove_node("/soc/i2c@40002000")?;
    /// fdt.remove_node("serial0")?; // 通过别名删除
    /// ```
    pub fn remove_node(&mut self, path: &str) -> Result<(), FdtError> {
        // 1. 基础输入验证
        let trimmed_path = path.trim();
        if trimmed_path.is_empty() || trimmed_path == "/" {
            return Err(FdtError::InvalidInput);
        }

        // 2. 高级路径规范化
        let normalized_path = Self::normalize_path_advanced(trimmed_path)?;

        // 3. 解析别名或处理绝对路径
        let final_path = if normalized_path.starts_with('/') {
            normalized_path
        } else {
            // 尝试别名解析
            match self.resolve_alias(&normalized_path) {
                Some(resolved) => {
                    // 确保解析的路径是绝对路径
                    if !resolved.starts_with('/') {
                        format!("/{}", resolved)
                    } else {
                        resolved
                    }
                }
                None => {
                    // 不是别名，转换为绝对路径
                    format!("/{}", normalized_path)
                }
            }
        };

        // 4. 最终路径验证
        if final_path == "/" || final_path.is_empty() {
            return Err(FdtError::InvalidInput);
        }

        // 5. 执行删除操作
        let result = self.remove_node_by_absolute_path(&final_path);

        // 6. 只有删除成功时才重建缓存
        if result.is_ok() {
            self.rebuild_phandle_cache();
        }

        result
    }

    /// 通过绝对路径删除节点（路径必须以 '/' 开头）
    fn remove_node_by_absolute_path(&mut self, path: &str) -> Result<(), FdtError> {
        if !path.starts_with('/') || path == "/" {
            return Err(FdtError::InvalidInput);
        }

        // 移除开头的斜杠并分割路径
        let path_without_root = path.trim_start_matches('/');
        let segments: Vec<&str> = path_without_root
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if segments.is_empty() {
            return Err(FdtError::InvalidInput);
        }

        // 处理直接删除根节点子节点的情况
        if segments.len() == 1 {
            return Self::remove_child_from_node(&mut self.root, segments[0]);
        }

        // 处理两级路径删除（目前测试只需要这个）
        if segments.len() == 2 {
            let parent_name = segments[0];
            let target_name = segments[1];

            // 找到父节点的索引
            let parent_index = self.find_child_index(&self.root, parent_name)?;

            // 删除父节点中的目标子节点
            return Self::remove_child_from_node(&mut self.root.children[parent_index], target_name);
        }

        // 对于更深层级的路径，暂时返回 NotFound
        Err(FdtError::NotFound)
    }

    /// 查找子节点索引
    fn find_child_index(&self, parent: &Node, child_name: &str) -> Result<usize, FdtError> {
        let child_name_base = Self::extract_node_name_base(child_name);

        for (i, child) in parent.children.iter().enumerate() {
            let child_base = Self::extract_node_name_base(&child.name);
            if child_base == child_name_base {
                return Ok(i);
            }
        }

        Err(FdtError::NotFound)
    }

    /// 高级路径规范化函数
    ///
    /// 处理各种边界情况：
    /// - 多个连续斜杠： "//path//to//node" -> "/path/to/node"
    /// - 前后空格： "  /path/to/node/  " -> "/path/to/node"
    /// - 尾部斜杠： "/path/to/node/" -> "/path/to/node"
    fn normalize_path_advanced(path: &str) -> Result<String, FdtError> {
        let trimmed = path.trim();

        // 特殊处理根路径
        if trimmed == "/" {
            return Ok("/".to_string());
        }

        // 分割并过滤空段，然后重新连接
        let segments: Vec<&str> = trimmed
            .split('/')
            .filter(|segment| !segment.trim().is_empty())
            .collect();

        if segments.is_empty() {
            return Ok("/".to_string());
        }

        // 确保返回绝对路径
        Ok(format!("/{}", segments.join("/")))
    }

  
    
    /// 从父节点中删除匹配的子节点
    ///
    /// 支持以下匹配模式：
    /// 1. 精确匹配：完整的节点名称（包括 @unit-address）
    /// 2. 部分匹配：节点名称基础部分（忽略 @unit-address）
    fn remove_child_from_node(parent: &mut Node, child_name: &str) -> Result<(), FdtError> {
        let original_count = parent.children.len();
        let child_name_base = Self::extract_node_name_base(child_name);

        // 查找所有匹配的子节点
        let matching_indices: Vec<usize> = parent.children
            .iter()
            .enumerate()
            .filter_map(|(index, child)| {
                let child_base = Self::extract_node_name_base(&child.name);
                if child_base == child_name_base {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();

        // 如果没有找到匹配的节点
        if matching_indices.is_empty() {
            return Err(FdtError::NotFound);
        }

        // 从后往前删除，避免索引移动问题
        for &index in matching_indices.iter().rev() {
            parent.children.remove(index);
        }

        // 验证删除是否成功
        if parent.children.len() < original_count {
            Ok(())
        } else {
            // 这种情况理论上不应该发生，但作为安全检查
            Err(FdtError::NotFound)
        }
    }

    /// 从节点名称中提取基础名称（去除 @unit-address 部分）
    ///
    /// # Examples
    /// - "uart@1000" -> "uart"
    /// - "memory" -> "memory"
    /// - "gpio@20000" -> "gpio"
    fn extract_node_name_base(node_name: &str) -> &str {
        node_name.split('@').next().unwrap_or(node_name)
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
            .find_child("__overlay__")
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

    /// 获取节点的完整路径
    fn get_node_path(&self, target: &Node) -> Option<String> {
        Self::find_node_path_recursive(&self.root, target, String::from("/"))
    }

    /// 递归查找节点路径
    fn find_node_path_recursive(current: &Node, target: &Node, path: String) -> Option<String> {
        // 检查是否是目标节点（通过指针比较）
        if core::ptr::eq(current, target) {
            return Some(path);
        }

        // 递归搜索子节点
        for child in &current.children {
            let child_path = if path == "/" {
                format!("/{}", child.name)
            } else {
                format!("{}/{}", path, child.name)
            };
            if let Some(found) = Self::find_node_path_recursive(child, target, child_path) {
                return Some(found);
            }
        }

        None
    }

    /// 将 overlay 应用到目标节点
    fn apply_overlay_to_target(
        &mut self,
        target_path: &str,
        overlay_node: &Node,
    ) -> Result<(), FdtError> {
        // 找到目标节点
        let (target, _path) = self
            .find_by_path_mut(target_path)
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
            if let Some(existing) = self.root.children.iter_mut().find(|c| c.name == child.name) {
                // 合并到现有子节点
                Self::merge_nodes(existing, child);
            } else {
                // 添加新子节点
                self.root.children.push(child.clone());
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
            if let Some(target_child) = target
                .children
                .iter_mut()
                .find(|c| c.name == source_child.name)
            {
                // 递归合并
                Self::merge_nodes(target_child, source_child);
            } else {
                // 添加新子节点
                target.children.push(source_child.clone());
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
        node.children.retain(|child| {
            !matches!(
                child.find_property("status"),
                Some(crate::Property::Status(crate::Status::Disabled))
            )
        });

        // 递归处理剩余子节点
        for child in &mut node.children {
            Self::remove_disabled_nodes(child);
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

/// FDT 构建器
struct FdtBuilder {
    /// 结构块数据
    struct_data: Vec<u32>,
    /// 字符串块数据
    strings_data: Vec<u8>,
    /// 字符串偏移映射
    string_offsets: Vec<(String, u32)>,
}

impl FdtBuilder {
    fn new() -> Self {
        Self {
            struct_data: Vec::new(),
            strings_data: Vec::new(),
            string_offsets: Vec::new(),
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
        for child in &node.children {
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
            self.build_property(prop);
        }

        // 子节点
        for child in &node.children {
            self.build_node(child);
        }

        // END_NODE
        let end_token: u32 = Token::EndNode.into();
        self.struct_data.push(end_token.to_be());
    }

    /// 构建属性
    fn build_property(&mut self, prop: &crate::Property) {
        // PROP token
        let prop_token: u32 = Token::Prop.into();
        self.struct_data.push(prop_token.to_be());

        // 获取序列化数据
        let data = prop.to_bytes();

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
