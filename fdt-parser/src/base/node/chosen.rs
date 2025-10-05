use core::{fmt::Debug, ops::Deref};

use crate::{base::NodeBase, FdtError};

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
    pub fn bootargs(&self) -> Result<&'a str, FdtError> {
        let prop = self.node.find_property("bootargs")?;
        prop.str()
    }

    /// Searches for the node representing `stdout`, if the property exists,
    /// attempting to resolve aliases if the node name doesn't exist as-is
    pub fn stdout(&self) -> Result<Stdout<'a>, FdtError> {
        let prop = self.node.find_property("stdout-path")?;

        let path = prop.str()?;

        let mut sp = path.split(':');

        let name = none_ok!(sp.next(), FdtError::NodeNotFound("path"));

        let params = sp.next();
        let node = self
            .node
            .fdt
            .find_nodes(name)
            .next()
            .ok_or(FdtError::NodeNotFound("path"))??;

        Ok(Stdout {
            params,
            node: node.deref().clone(),
        })
    }

    pub fn debugcon(&self) -> Result<DebugCon<'a>, FdtError> {
        match self.stdout() {
            Ok(stdout) => Ok(DebugCon::Node(stdout.node.clone())),
            Err(FdtError::NotFound) | Err(FdtError::NodeNotFound(_)) => {
                self.fdt_bootargs_find_debugcon_info()
            }
            Err(e) => Err(e),
        }
    }

    fn fdt_bootargs_find_debugcon_info(&self) -> Result<DebugCon<'a>, FdtError> {
        let bootargs = self.bootargs()?;

        let earlycon = none_ok!(bootargs
            .split_ascii_whitespace()
            .find(|&arg| arg.contains("earlycon")));

        let mut tmp = earlycon.split('=');
        let _ = none_ok!(tmp.next(), FdtError::NotFound);
        let values = none_ok!(tmp.next(), FdtError::NotFound);

        // 解析所有参数
        let mut params_iter = values.split(',');
        let name = none_ok!(params_iter.next(), FdtError::NotFound);

        if !name.contains("uart") {
            return Err(FdtError::NotFound);
        }

        let param2 = none_ok!(params_iter.next(), FdtError::NotFound);

        let addr_str = if param2.contains("0x") {
            param2
        } else {
            none_ok!(params_iter.next(), FdtError::NotFound)
        };

        let mmio = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16)
            .map_err(|_| FdtError::Utf8Parse)?;

        // 先尝试在设备树中查找对应节点
        for node_result in self.node.fdt.all_nodes() {
            let node = node_result?;
            match node.reg() {
                Ok(mut regs) => {
                    for reg in &mut regs {
                        if reg.address == mmio {
                            return Ok(DebugCon::Node(node.node().clone()));
                        }
                    }
                }
                Err(FdtError::NotFound) => {}
                Err(e) => return Err(e),
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

        Ok(DebugCon::EarlyConInfo { name, mmio, params })
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
