//! DTS 格式化显示模块
//!
//! 提供将 FDT 结构格式化为 DTS 源文件格式的功能

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use crate::prop::PropertyKind;
use crate::{Fdt, Node, NodeRef, Property, node::NodeOp};

/// 带层级缩进的格式化 trait
pub trait FmtLevel {
    /// 使用指定的缩进深度格式化输出
    fn fmt_level(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result;
}

/// 获取缩进字符串
fn indent(level: usize) -> String {
    "    ".repeat(level)
}

// ============================================================================
// Property 实现
// ============================================================================

impl FmtLevel for Property {
    fn fmt_level(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        let indent_str = indent(level);
        write!(f, "{}{}", indent_str, self.format_dts())
    }
}

impl Property {
    /// 格式化属性为 DTS 格式字符串
    pub fn format_dts(&self) -> String {
        match &self.kind {
            PropertyKind::Bool => {
                format!("{};", self.name)
            }
            PropertyKind::Num(v) => {
                format!("{} = <{:#x}>;", self.name, v)
            }
            PropertyKind::NumVec(values) => {
                let values_str: String = values
                    .iter()
                    .map(|v| format!("{:#x}", v))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} = <{}>;", self.name, values_str)
            }
            PropertyKind::Str(s) => {
                format!("{} = \"{}\";", self.name, escape_string(s))
            }
            PropertyKind::StringList(strs) => {
                let strs_fmt: String = strs
                    .iter()
                    .map(|s| format!("\"{}\"", escape_string(s)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} = {};", self.name, strs_fmt)
            }
            PropertyKind::Status(status) => {
                let s = match status {
                    fdt_raw::Status::Okay => "okay",
                    fdt_raw::Status::Disabled => "disabled",
                };
                format!("{} = \"{}\";", self.name, s)
            }
            PropertyKind::Phandle(ph) => {
                format!("{} = <{:#x}>;", self.name, ph.as_usize())
            }
            PropertyKind::Reg(entries) => {
                let entries_str: String = entries
                    .iter()
                    .map(|e| {
                        if let Some(size) = e.size {
                            format!("{:#x} {:#x}", e.address, size)
                        } else {
                            format!("{:#x}", e.address)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} = <{}>;", self.name, entries_str)
            }
            PropertyKind::Raw(raw) => format_raw_property(&self.name, raw.data()),
            PropertyKind::Clocks(clock_refs) => {
                // 格式化 clocks 属性为 <phandle specifier...> 格式
                let clocks_str: String = clock_refs
                    .iter()
                    .map(|cr| {
                        let mut parts = vec![format!("{:#x}", cr.phandle.as_usize())];
                        for s in &cr.specifier {
                            parts.push(format!("{:#x}", s));
                        }
                        parts.join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} = <{}>;", self.name, clocks_str)
            }
        }
    }
}

/// 转义字符串中的特殊字符
fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

/// 格式化原始属性数据
fn format_raw_property(name: &str, data: &[u8]) -> String {
    if data.is_empty() {
        return format!("{};", name);
    }

    // 尝试解析为字符串
    if let Some(s) = try_parse_string(data) {
        return format!("{} = \"{}\";", name, escape_string(&s));
    }

    // 尝试解析为字符串列表
    if let Some(strs) = try_parse_string_list(data) {
        let strs_fmt: String = strs
            .iter()
            .map(|s| format!("\"{}\"", escape_string(s)))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{} = {};", name, strs_fmt);
    }

    // 如果是 4 字节对齐，尝试解析为 u32 数组
    if data.len().is_multiple_of(4) {
        let values: Vec<String> = data
            .chunks(4)
            .map(|chunk| {
                let v = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                format!("{:#x}", v)
            })
            .collect();
        return format!("{} = <{}>;", name, values.join(" "));
    }

    // 否则作为字节数组输出
    let bytes: Vec<String> = data.iter().map(|b| format!("{:02x}", b)).collect();
    format!("{} = [{}];", name, bytes.join(" "))
}

/// 尝试解析为单个字符串
fn try_parse_string(data: &[u8]) -> Option<String> {
    // 必须以 null 结尾
    if data.is_empty() || data[data.len() - 1] != 0 {
        return None;
    }

    // 检查是否只有一个 null 终止符（在末尾）
    let null_count = data.iter().filter(|&&b| b == 0).count();
    if null_count != 1 {
        return None;
    }

    // 尝试解析为 UTF-8
    let str_bytes = &data[..data.len() - 1];
    core::str::from_utf8(str_bytes).ok().map(|s| s.to_string())
}

/// 尝试解析为字符串列表
fn try_parse_string_list(data: &[u8]) -> Option<Vec<String>> {
    // 必须以 null 结尾
    if data.is_empty() || data[data.len() - 1] != 0 {
        return None;
    }

    let mut result = Vec::new();
    let mut start = 0;

    for (i, &byte) in data.iter().enumerate() {
        if byte == 0 {
            if i > start {
                let str_bytes = &data[start..i];
                match core::str::from_utf8(str_bytes) {
                    Ok(s) => result.push(s.to_string()),
                    Err(_) => return None,
                }
            }
            start = i + 1;
        }
    }

    if result.len() > 1 { Some(result) } else { None }
}

// ============================================================================
// Node 实现
// ============================================================================

impl FmtLevel for Node {
    fn fmt_level(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        let indent_str = indent(level);

        // 节点名（根节点名为空，显示为 /）
        let node_name = if self.name().is_empty() {
            "/"
        } else {
            self.name()
        };

        // 如果没有属性和子节点，写成单行
        if self.properties().count() == 0 && self.children().count() == 0 {
            return writeln!(f, "{}{} {{ }};", indent_str, node_name);
        }

        writeln!(f, "{}{} {{", indent_str, node_name)?;

        // 写入属性
        for prop in self.properties() {
            prop.fmt_level(f, level + 1)?;
            writeln!(f)?;
        }

        // 如果有子节点，添加空行分隔
        if self.properties().count() > 0 && self.children().count() > 0 {
            writeln!(f)?;
        }

        // 写入子节点
        for child in self.children() {
            child.fmt_level(f, level + 1)?;
        }

        // 写入节点结束
        writeln!(f, "{}}};", indent_str)
    }
}

// ============================================================================
// NodeRef 实现
// ============================================================================

impl<'a> FmtLevel for NodeRef<'a> {
    fn fmt_level(&self, f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
        self.node.fmt_level(f, level)
    }
}

// ============================================================================
// Fdt Display 实现
// ============================================================================

impl fmt::Display for Fdt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 写入 DTS 文件头
        writeln!(f, "/dts-v1/;")?;

        // 写入内存保留块
        for rsv in &self.memory_reservations {
            writeln!(f, "/memreserve/ {:#x} {:#x};", rsv.address, rsv.size)?;
        }

        if !self.memory_reservations.is_empty() {
            writeln!(f)?;
        }

        // 使用 FmtLevel 输出根节点
        self.root.fmt_level(f, 0)
    }
}
