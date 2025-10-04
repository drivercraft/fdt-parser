use core::{fmt::Debug, ops::Deref};

use crate::{
    base::NodeBase,
    FdtError,
};

#[derive(Clone, Debug)]
pub enum DebugCon<'a> {
    /// 找到了对应的设备树节点
    Node(NodeBase<'a>),
    /// 仅在bootargs中找到earlycon参数，包含解析出的信息
    EarlyConInfo {
        name: &'a str,
        mmio: u64,
        params: Option<&'a str>,
    },
}

#[derive(Clone)]
pub struct Chosen<'a> {
    node: NodeBase<'a>,
}

impl<'a> Chosen<'a> {
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        Chosen { node }
    }

    /// Contains the bootargs, if they exist
    pub fn bootargs(&self) -> Result<Option<&'a str>, FdtError> {
        let prop = self.node.find_property("bootargs")?;
        match prop {
            Some(p) => Ok(Some(p.str()?)),
            None => Ok(None),
        }
    }

    /// Searches for the node representing `stdout`, if the property exists,
    /// attempting to resolve aliases if the node name doesn't exist as-is
    pub fn stdout(&self) -> Result<Option<Stdout<'a>>, FdtError> {
        let prop = none_ok!(self.node.find_property("stdout-path")?);

        let path = prop.str()?;

        let mut sp = path.split(':');

        let name = none_ok!(sp.next());

        let params = sp.next();
        let node = self
            .node
            .fdt
            .find_nodes(name)
            .next()
            .ok_or(FdtError::NodeNotFound("path"))??;

        Ok(Some(Stdout {
            params,
            node: node.deref().clone(),
        }))
    }

    pub fn debugcon(&self) -> Result<Option<DebugCon<'a>>, FdtError> {
        if let Some(node) = self.stdout()? {
            Ok(Some(DebugCon::Node(node.node)))
        } else {
            self.fdt_bootargs_find_debugcon_info()
        }
    }

    fn fdt_bootargs_find_debugcon_info(&self) -> Result<Option<DebugCon<'a>>, FdtError> {
        let bootargs = none_ok!(self.bootargs()?);

        let earlycon = none_ok!(bootargs
            .split_ascii_whitespace()
            .find(|&arg| arg.contains("earlycon")));

        let mut tmp = earlycon.split('=');
        let _ = none_ok!(tmp.next());
        let values = none_ok!(tmp.next());

        // 解析所有参数
        let mut params_iter = values.split(',');
        let name = none_ok!(params_iter.next());

        if !name.contains("uart") {
            return Ok(None);
        }

        let param2 = none_ok!(params_iter.next());

        let addr_str = if param2.contains("0x") {
            param2
        } else {
            none_ok!(params_iter.next())
        };

        let mmio = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16)
            .map_err(|_| FdtError::Utf8Parse)?;

        // 先尝试在设备树中查找对应节点
        for node_result in self.node.fdt.all_nodes() {
            let node = node_result?;
            if let Some(regs) = node.reg()? {
                for reg in regs {
                    if reg.address.eq(&mmio) {
                        return Ok(Some(DebugCon::Node((*node).clone())));
                    }
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
                Some(&values[param_start..])
            } else {
                Some(param)
            }
        } else {
            None
        };

        Ok(Some(DebugCon::EarlyConInfo {
            name,
            mmio,
            params,
        }))
    }
}

impl Debug for Chosen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Chosen")
            .field("bootargs", &self.bootargs())
            .field("stdout", &self.stdout())
            .finish()
    }
}

impl<'a> Deref for Chosen<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

#[derive(Clone)]
pub struct Stdout<'a> {
    pub params: Option<&'a str>,
    pub node: NodeBase<'a>,
}

impl<'a> Stdout<'a> {}

impl Debug for Stdout<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Stdout")
            .field("name", &self.node.name())
            .field("params", &self.params)
            .finish()
    }
}

impl<'a> Deref for Stdout<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
