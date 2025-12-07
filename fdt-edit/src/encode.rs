//! FDT 编码模块
//!
//! 将 Fdt 结构序列化为 DTB 二进制格式

use alloc::{string::String, vec::Vec};
use core::ops::Deref;
use fdt_raw::{FDT_MAGIC, Token};

use crate::{Context, Fdt, Node};

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

/// 节点编码 trait
pub trait NodeEncode {
    /// 编码节点到 encoder
    fn encode(&self, encoder: &mut FdtEncoder, ctx: &Context<'_>);
}

/// FDT 编码器
pub struct FdtEncoder<'a> {
    /// FDT 引用
    fdt: &'a Fdt,
    /// 结构块数据
    struct_data: Vec<u32>,
    /// 字符串块数据
    strings_data: Vec<u8>,
    /// 字符串偏移映射
    string_offsets: Vec<(String, u32)>,
}

impl<'a> FdtEncoder<'a> {
    /// 创建新的编码器
    pub fn new(fdt: &'a Fdt) -> Self {
        Self {
            fdt,
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

    /// 写入 BEGIN_NODE token
    fn write_begin_node(&mut self, name: &str) {
        let begin_token: u32 = Token::BeginNode.into();
        self.struct_data.push(begin_token.to_be());

        // 节点名（包含 null 终止符，对齐到 4 字节）
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() + 1; // +1 for null
        let aligned_len = (name_len + 3) & !3;

        let mut name_buf = vec![0u8; aligned_len];
        name_buf[..name_bytes.len()].copy_from_slice(name_bytes);

        // 转换为 u32 数组（保持字节顺序不变）
        for chunk in name_buf.chunks(4) {
            let word = u32::from_ne_bytes(chunk.try_into().unwrap());
            self.struct_data.push(word);
        }
    }

    /// 写入 END_NODE token
    pub fn write_end_node(&mut self) {
        let end_token: u32 = Token::EndNode.into();
        self.struct_data.push(end_token.to_be());
    }

    /// 写入属性
    pub fn write_property(&mut self, name: &str, data: &[u8]) {
        // PROP token
        let prop_token: u32 = Token::Prop.into();
        self.struct_data.push(prop_token.to_be());

        // 属性长度
        self.struct_data.push((data.len() as u32).to_be());

        // 字符串偏移
        let nameoff = self.get_or_add_string(name);
        self.struct_data.push(nameoff.to_be());

        // 属性数据（对齐到 4 字节）
        if !data.is_empty() {
            let aligned_len = (data.len() + 3) & !3;
            let mut data_buf = vec![0u8; aligned_len];
            data_buf[..data.len()].copy_from_slice(data);

            // 转换为 u32 数组（保持字节顺序不变）
            for chunk in data_buf.chunks(4) {
                let word = u32::from_ne_bytes(chunk.try_into().unwrap());
                self.struct_data.push(word);
            }
        }
    }

    /// 执行编码
    pub fn encode(mut self) -> FdtData {
        // 收集所有字符串
        self.collect_strings(&self.fdt.root.clone());

        // 构建 phandle 映射
        let mut phandle_map = alloc::collections::BTreeMap::new();
        Context::build_phandle_map_from_node(&self.fdt.root, &mut phandle_map);

        // 创建遍历上下文
        let mut ctx = Context::new();
        ctx.set_phandle_map(phandle_map);

        // 编码根节点
        self.encode_node(&self.fdt.root.clone(), &ctx);

        // 添加 END token
        let token: u32 = Token::End.into();
        self.struct_data.push(token.to_be());

        // 生成最终数据
        self.finalize()
    }

    /// 递归收集所有属性名字符串
    fn collect_strings(&mut self, node: &Node) {
        for prop in node.properties() {
            self.get_or_add_string(prop.name());
        }
        for child in node.children() {
            self.collect_strings(child);
        }
    }

    /// 编码节点
    fn encode_node(&mut self, node: &Node, ctx: &Context<'_>) {
        // 调用节点的 encode 方法
        node.encode(self, ctx);

        // 编码子节点
        for child in node.children() {
            let child_ctx = ctx.for_child(node);
            self.encode_node(child, &child_ctx);
        }

        // 写入 END_NODE
        self.write_end_node();
    }

    /// 生成最终 FDT 数据
    fn finalize(self) -> FdtData {
        let memory_reservations = &self.fdt.memory_reservations;
        let boot_cpuid_phys = self.fdt.boot_cpuid_phys;

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

/// 为 Node 实现编码
impl NodeEncode for Node {
    fn encode(&self, encoder: &mut FdtEncoder, ctx: &FdtContext) {
        // 写入 BEGIN_NODE
        encoder.write_begin_node(self.name());

        // 编码所有属性，使用父节点的 cells 信息
        for prop in self.properties() {
            let data = prop.encode(ctx);
            encoder.write_property(prop.name(), &data);
        }
    }
}
