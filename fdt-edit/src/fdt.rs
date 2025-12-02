use core::ops::Deref;

use alloc::{
    collections::BTreeMap,
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

    /// 根据路径查找节点
    ///
    /// 路径格式: "/node1/node2/node3"
    /// 支持 alias：如果路径不以 '/' 开头，会从 /aliases 节点解析别名
    pub fn find_by_path(&self, path: &str) -> Option<&Node> {
        // 如果路径以 '/' 开头，直接按路径查找
        // 否则解析 alias
        let resolved_path = if path.starts_with('/') {
            path.to_string()
        } else {
            self.resolve_alias(path)?
        };

        let path = resolved_path.trim_start_matches('/');
        if path.is_empty() {
            return Some(&self.root);
        }

        let mut current = &self.root;
        for part in path.split('/') {
            if part.is_empty() {
                continue;
            }
            current = current.find_child(part)?;
        }
        Some(current)
    }

    /// 根据路径查找节点（可变）
    ///
    /// 支持 alias：如果路径不以 '/' 开头，会从 /aliases 节点解析别名
    pub fn find_by_path_mut(&mut self, path: &str) -> Option<&mut Node> {
        // 如果路径以 '/' 开头，直接按路径查找
        // 否则解析 alias
        let resolved_path = if path.starts_with('/') {
            path.to_string()
        } else {
            self.resolve_alias(path)?
        };

        let path = resolved_path.trim_start_matches('/');
        if path.is_empty() {
            return Some(&mut self.root);
        }

        let mut current = &mut self.root;
        for part in path.split('/') {
            if part.is_empty() {
                continue;
            }
            current = current.find_child_mut(part)?;
        }
        Some(current)
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
    pub fn find_all_by_path(&self, path: &str) -> Vec<&Node> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return vec![&self.root];
        }

        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return vec![&self.root];
        }

        let mut results = Vec::new();
        Self::find_all_by_path_recursive(&self.root, &parts, 0, &mut results);
        results
    }

    /// 递归查找路径匹配的节点
    fn find_all_by_path_recursive<'a>(
        node: &'a Node,
        parts: &[&str],
        index: usize,
        results: &mut Vec<&'a Node>,
    ) {
        if index >= parts.len() {
            results.push(node);
            return;
        }

        let part = parts[index];
        if part == "*" {
            // 通配符：遍历所有子节点
            for child in &node.children {
                Self::find_all_by_path_recursive(child, parts, index + 1, results);
            }
        } else {
            // 精确匹配：可能有多个同名节点
            for child in &node.children {
                if child.name == part {
                    Self::find_all_by_path_recursive(child, parts, index + 1, results);
                }
            }
        }
    }

    /// 根据节点名称查找所有匹配的节点（递归搜索整个树）
    ///
    /// 返回所有名称匹配的节点引用
    pub fn find_by_name(&self, name: &str) -> Vec<&Node> {
        let mut results = Vec::new();
        Self::find_by_name_recursive(&self.root, name, &mut results);
        results
    }

    /// 递归按名称查找节点
    fn find_by_name_recursive<'a>(node: &'a Node, name: &str, results: &mut Vec<&'a Node>) {
        // 检查当前节点
        if node.name == name {
            results.push(node);
        }

        // 递归检查所有子节点
        for child in &node.children {
            Self::find_by_name_recursive(child, name, results);
        }
    }

    /// 根据节点名称前缀查找所有匹配的节点
    ///
    /// 例如: find_by_name_prefix("gpio") 会匹配 "gpio", "gpio0", "gpio@1000" 等
    pub fn find_by_name_prefix(&self, prefix: &str) -> Vec<&Node> {
        let mut results = Vec::new();
        Self::find_by_name_prefix_recursive(&self.root, prefix, &mut results);
        results
    }

    /// 递归按名称前缀查找节点
    fn find_by_name_prefix_recursive<'a>(
        node: &'a Node,
        prefix: &str,
        results: &mut Vec<&'a Node>,
    ) {
        // 检查当前节点
        if node.name.starts_with(prefix) {
            results.push(node);
        }

        // 递归检查所有子节点
        for child in &node.children {
            Self::find_by_name_prefix_recursive(child, prefix, results);
        }
    }

    /// 根据 phandle 查找节点
    pub fn find_by_phandle(&self, phandle: Phandle) -> Option<&Node> {
        let index = self.phandle_cache.get(&phandle)?;
        self.get_node_by_index(index)
    }

    /// 根据 phandle 查找节点（可变）
    pub fn find_by_phandle_mut(&mut self, phandle: Phandle) -> Option<&mut Node> {
        let index = self.phandle_cache.get(&phandle)?.clone();
        self.get_node_by_index_mut(&index)
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
