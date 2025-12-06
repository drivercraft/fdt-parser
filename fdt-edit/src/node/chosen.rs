use alloc::string::ToString;

use super::{NodeOp, NodeTrait, RawNode};
use crate::{Property, prop::PropertyKind};

/// Chosen 节点，包含启动参数等信息
#[derive(Clone, Debug)]
pub struct NodeChosen(pub(crate) RawNode);

impl NodeOp for NodeChosen {}

impl NodeTrait for NodeChosen {
    fn as_raw(&self) -> &RawNode {
        &self.0
    }

    fn as_raw_mut(&mut self) -> &mut RawNode {
        &mut self.0
    }

    fn to_raw(self) -> RawNode {
        self.0
    }
}

impl Default for NodeChosen {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeChosen {
    pub fn new() -> Self {
        NodeChosen(RawNode::new("chosen"))
    }

    /// 获取 bootargs 属性
    pub fn bootargs(&self) -> Option<&str> {
        self.find_property_str("bootargs")
    }

    /// 获取 stdout-path 属性
    pub fn stdout_path(&self) -> Option<&str> {
        self.find_property_str("stdout-path")
    }

    /// 获取 stdin-path 属性
    pub fn stdin_path(&self) -> Option<&str> {
        self.find_property_str("stdin-path")
    }

    pub fn set_bootargs(&mut self, args: Option<&str>) {
        if let Some(args) = args {
            self.0.set_property(Property {
                name: "bootargs".to_string(),
                kind: PropertyKind::Str(args.to_string()),
            });
        } else {
            self.0.remove_property("bootargs");
        }
    }

    pub fn set_stdout_path(&mut self, path: Option<&str>) {
        if let Some(path) = path {
            self.0.set_property(Property {
                name: "stdout-path".to_string(),
                kind: PropertyKind::Str(path.to_string()),
            });
        } else {
            self.0.remove_property("stdout-path");
        }
    }

    pub fn set_stdin_path(&mut self, path: Option<&str>) {
        if let Some(path) = path {
            self.0.set_property(Property {
                name: "stdin-path".to_string(),
                kind: PropertyKind::Str(path.to_string()),
            });
        } else {
            self.0.remove_property("stdin-path");
        }
    }

    /// 查找字符串属性
    fn find_property_str(&self, name: &str) -> Option<&str> {
        let prop = self.find_property(name)?;
        match &prop.kind {
            PropertyKind::Str(s) => Some(s.as_str()),
            PropertyKind::Raw(raw) => raw.as_str(),
            _ => None,
        }
    }
}
