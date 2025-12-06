use core::ops::Deref;

use super::NodeBase;

/// 内存区域信息
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// 起始地址
    pub address: u64,
    /// 区域大小
    pub size: u64,
}

/// Memory 节点，描述物理内存布局
#[derive(Clone)]
pub struct Memory<'a> {
    node: NodeBase<'a>,
}

impl<'a> Memory<'a> {
    pub(crate) fn new(node: NodeBase<'a>) -> Self {
        Self { node }
    }

    /// 获取内存区域迭代器
    ///
    /// Memory 节点的 reg 属性描述了物理内存的布局
    pub fn regions(&self) -> impl Iterator<Item = MemoryRegion> + 'a {
        self.node.reg().into_iter().flat_map(|reg| {
            reg.iter().map(|info| MemoryRegion {
                address: info.address,
                size: info.size.unwrap_or(0),
            })
        })
    }

    /// 获取所有内存区域（使用固定大小数组）
    pub fn regions_array<const N: usize>(&self) -> heapless::Vec<MemoryRegion, N> {
        let mut result = heapless::Vec::new();
        for region in self.regions() {
            if result.push(region).is_err() {
                break;
            }
        }
        result
    }

    /// 计算总内存大小
    pub fn total_size(&self) -> u64 {
        self.regions().map(|r| r.size).sum()
    }
}

impl<'a> Deref for Memory<'a> {
    type Target = NodeBase<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl core::fmt::Debug for Memory<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut st = f.debug_struct("Memory");
        st.field("name", &self.node.name());
        for region in self.regions() {
            st.field("region", &region);
        }
        st.finish()
    }
}
