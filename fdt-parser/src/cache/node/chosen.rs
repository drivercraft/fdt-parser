use core::{fmt::Debug, ops::Deref};

use crate::cache::node::NodeBase;
use alloc::{string::String, string::ToString};

#[derive(Clone)]
pub struct Chosen {
    node: NodeBase,
}

impl Chosen {
    pub(crate) fn new(node: NodeBase) -> Self {
        Chosen { node }
    }

    /// Contains the bootargs, if they exist
    pub fn bootargs(&self) -> Option<String> {
        self.node
            .find_property("bootargs")
            .and_then(|prop| prop.str().ok())
            .map(|s| s.to_string())
    }

    /// Searches for the node representing `stdout`, if the property exists,
    /// attempting to resolve aliases if the node name doesn't exist as-is
    pub fn stdout(&self) -> Option<Stdout> {
        let prop = self.node.find_property("stdout-path")?;
        let path = prop.str().ok()?;

        let mut sp = path.split(':');
        let name = sp.next()?;
        let params = sp.next();

        // 尝试在cache中找到节点
        self.node.fdt.get_node_by_path(name).map(|node| Stdout {
                params: params.map(|s| s.to_string()),
                node,
            })
    }

    pub fn debugcon(&self) -> Option<DebugConCache> {
        if let Some(stdout) = self.stdout() {
            Some(DebugConCache::Node(stdout.node))
        } else {
            self.fdt_bootargs_find_debugcon_info()
        }
    }

    fn fdt_bootargs_find_debugcon_info(&self) -> Option<DebugConCache> {
        let bootargs = self.bootargs()?;

        // 查找 earlycon 参数
        let earlycon = bootargs
            .split_ascii_whitespace()
            .find(|arg| arg.contains("earlycon"))?;

        let mut tmp = earlycon.split('=');
        let _ = tmp.next()?; // 跳过 "earlycon"
        let values = tmp.next()?;

        // 解析所有参数
        let mut params_iter = values.split(',');
        let name = params_iter.next()?;

        if !name.contains("uart") {
            return None;
        }

        let param2 = params_iter.next()?;

        let addr_str = if param2.contains("0x") {
            param2
        } else {
            params_iter.next()?
        };

        let mmio = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16).ok()?;

        // 先尝试在cache中查找对应节点
        let all_nodes = self.node.fdt.all_nodes();
        for node in all_nodes {
            let Ok(reg) = node.reg() else {
                continue;
            };

            for address in reg {
                if address.address == mmio {
                    return Some(DebugConCache::Node(node));
                }
            }
        }

        // 如果找不到对应节点，返回解析出的earlycon信息
        // 重新分割字符串以获取剩余参数
        let mut parts = values.split(',');
        let _name = parts.next(); // 跳过name
        let _addr_part = parts.next(); // 跳过地址部分
        let params = if let Some(param) = parts.next() {
            // 获取第一个剩余参数的位置，然后取剩余所有内容
            let param_start = values.find(param).unwrap_or(0);
            if param_start > 0 {
                Some(values[param_start..].to_string())
            } else {
                Some(param.to_string())
            }
        } else {
            None
        };

        Some(DebugConCache::EarlyConInfo {
            name: name.to_string(),
            mmio,
            params,
        })
    }
}

impl Debug for Chosen {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Chosen")
            .field("bootargs", &self.bootargs())
            .field("stdout", &self.stdout())
            .finish()
    }
}

impl Deref for Chosen {
    type Target = NodeBase;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

#[derive(Clone, Debug)]
pub enum DebugConCache {
    /// 找到了对应的设备树节点
    Node(super::super::Node),
    /// 仅在bootargs中找到earlycon参数，包含解析出的信息
    EarlyConInfo {
        name: String,
        mmio: u64,
        params: Option<String>,
    },
}

#[derive(Clone)]
pub struct Stdout {
    pub params: Option<String>,
    pub node: super::super::Node,
}

impl Stdout {}

impl Debug for Stdout {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Stdout")
            .field("name", &self.node.name())
            .field("params", &self.params)
            .finish()
    }
}
