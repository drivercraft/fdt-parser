use core::ops::Deref;

use super::NodeBase;

/// Chosen 节点，包含启动参数等信息
#[derive(Clone)]
pub struct Chosen<'a> {
    node: NodeBase<'a>,
}

impl<'a> Chosen<'a> {
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        Self { node }
    }

    /// 获取 bootargs 属性
    pub fn bootargs(&self) -> Option<&'a str> {
        self.node.find_property_str("bootargs")
    }

    /// 获取 stdout-path 属性
    pub fn stdout_path(&self) -> Option<&'a str> {
        self.node.find_property_str("stdout-path")
    }

    /// 获取 stdin-path 属性
    pub fn stdin_path(&self) -> Option<&'a str> {
        self.node.find_property_str("stdin-path")
    }
}

impl<'a> Deref for Chosen<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl core::fmt::Debug for Chosen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Chosen")
            .field("bootargs", &self.bootargs())
            .field("stdout_path", &self.stdout_path())
            .finish()
    }
}
