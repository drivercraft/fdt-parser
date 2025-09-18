use crate::{
    data::{Buffer, Raw},
    property::PropIter,
    Fdt, FdtError, FdtRangeSilce, FdtReg, Phandle, Property,
};

mod chosen;
mod memory;

pub use chosen::*;
pub use memory::*;

#[derive(Clone)]
pub struct Node<'a> {
    name: &'a str,
    pub(crate) fdt: Fdt<'a>,
    pub level: usize,
    pub(crate) raw: Raw<'a>,
    pub(crate) parent: Option<ParentInfo<'a>>,
}

#[derive(Clone)]
pub(crate) struct ParentInfo<'a> {
    name: &'a str,
    level: usize,
    raw: Raw<'a>,
    parent_address_cell: Option<u8>,
    parent_size_cell: Option<u8>,
    parent_name: Option<&'a str>,
}

impl<'a> Node<'a> {
    pub(crate) fn new(
        name: &'a str,
        fdt: Fdt<'a>,
        raw: Raw<'a>,
        level: usize,
        parent: Option<&Node<'a>>,
    ) -> Self {
        let name = if name.is_empty() { "/" } else { name };
        Node {
            name,
            fdt,
            level,
            parent: parent.map(|p| {
                let pp = p.parent_fast();

                let parent_address_cell = pp.as_ref().and_then(|pn| {
                    pn.find_property("#address-cells")
                        .ok()
                        .flatten()
                        .and_then(|prop| prop.u32().ok())
                        .map(|v| v as u8)
                });
                let parent_size_cell = pp.as_ref().and_then(|pn| {
                    pn.find_property("#size-cells")
                        .ok()
                        .flatten()
                        .and_then(|prop| prop.u32().ok())
                        .map(|v| v as u8)
                });

                ParentInfo {
                    name: p.name(),
                    level: p.level(),
                    raw: p.raw(),
                    parent_address_cell,
                    parent_size_cell,
                    parent_name: pp.as_ref().and_then(|pn| pn.parent_name()),
                }
            }),
            raw,
        }
    }

    pub fn parent_name(&self) -> Option<&'a str> {
        self.parent_fast().map(|p| p.name())
    }

    pub fn parent(&self) -> Option<Node<'a>> {
        let parent_name = self.parent_name()?;
        self.fdt
            .all_nodes()
            .flatten()
            .find(|node| node.name() == parent_name)
    }

    fn parent_fast(&self) -> Option<Node<'a>> {
        self.parent.as_ref().map(|p| Node {
            name: p.name,
            fdt: self.fdt.clone(),
            level: p.level,
            raw: p.raw,
            parent: None,
        })
    }

    pub fn raw(&self) -> Raw<'a> {
        self.raw
    }

    /// Get the name of this node
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Get the level/depth of this node in the device tree
    pub fn level(&self) -> usize {
        self.level
    }

    /// Get compatible strings for this node (placeholder implementation)
    pub fn compatibles(&self) -> Result<Option<impl Iterator<Item = &'a str> + 'a>, FdtError> {
        let prop = self.find_property("compatible")?;
        Ok(prop.map(|p| p.str_list()))
    }

    pub fn compatibles_flatten(&self) -> impl Iterator<Item = &'a str> + 'a {
        self.compatibles().ok().flatten().into_iter().flatten()
    }

    pub fn reg(&self) -> Result<Option<RegIter<'a>>, FdtError> {
        let pp_address_cell = self.parent.as_ref().and_then(|p| p.parent_address_cell);

        let prop = none_ok!(self.find_property("reg")?);

        // Use full parent resolution to retain its ancestor chain
        let parent = self.parent_fast().ok_or(FdtError::NodeNotFound("parent"))?;

        // reg parsing uses the immediate parent's cells
        let address_cell = parent
            .find_property("#address-cells")?
            .ok_or(FdtError::PropertyNotFound("#address-cells"))?
            .u32()? as u8;

        let size_cell = parent
            .find_property("#size-cells")?
            .ok_or(FdtError::PropertyNotFound("#size-cells"))?
            .u32()? as u8;

        let ranges = parent.node_ranges(pp_address_cell)?;

        Ok(Some(RegIter {
            size_cell,
            address_cell,
            buff: prop.data.buffer(),
            ranges,
        }))
    }

    pub fn to_kind(self) -> NodeKind<'a> {
        if self.name() == "chosen" {
            NodeKind::Chosen(Chosen::new(self))
        } else if self.name().starts_with("memory") {
            NodeKind::Memory(Memory::new(self))
        } else {
            NodeKind::General(self)
        }
    }

    /// 检查这个节点是否是根节点
    pub fn is_root(&self) -> bool {
        self.level == 0
    }

    /// 获取节点的完整路径信息（仅限调试用途）
    pub fn debug_info(&self) -> NodeDebugInfo<'a> {
        NodeDebugInfo {
            name: self.name(),
            level: self.level,
            pos: self.raw.pos(),
        }
    }

    pub fn properties(&self) -> impl Iterator<Item = Result<Property<'a>, FdtError>> + '_ {
        let reader = self.raw.buffer();
        PropIter::new(self.fdt.clone(), reader)
    }

    pub fn find_property(&self, name: &str) -> Result<Option<Property<'a>>, FdtError> {
        for prop in self.properties() {
            let prop = prop?;
            if prop.name.eq(name) {
                return Ok(Some(prop));
            }
        }
        Ok(None)
    }

    pub(crate) fn node_ranges(
        &self,
        address_cell_parent: Option<u8>,
    ) -> Result<Option<FdtRangeSilce<'a>>, FdtError> {
        let prop = none_ok!(self.find_property("ranges")?);

        let address_cell = self
            .find_property("#address-cells")?
            .ok_or(FdtError::PropertyNotFound("#address-cells"))?
            .u32()? as u8;
        let size_cell = self
            .find_property("#size-cells")?
            .ok_or(FdtError::PropertyNotFound("#size-cells"))?
            .u32()? as u8;
        let address_cell_parent = address_cell_parent
            .ok_or(FdtError::PropertyNotFound("parent.parent.#address-cells"))?;

        Ok(Some(FdtRangeSilce::new(
            address_cell,
            address_cell_parent,
            size_cell,
            prop.data.buffer(),
        )))
    }

    pub fn phandle(&self) -> Result<Option<Phandle>, FdtError> {
        let prop = self.find_property("phandle")?;
        match prop {
            Some(p) => Ok(Some(p.u32()?.into())),
            None => Ok(None),
        }
    }
}

/// 节点调试信息
#[derive(Debug)]
pub struct NodeDebugInfo<'a> {
    pub name: &'a str,
    pub level: usize,
    pub pos: usize,
}

impl core::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Node").field("name", &self.name()).finish()
    }
}

pub struct RegIter<'a> {
    size_cell: u8,
    address_cell: u8,
    buff: Buffer<'a>,
    ranges: Option<FdtRangeSilce<'a>>,
}
impl Iterator for RegIter<'_> {
    type Item = FdtReg;

    fn next(&mut self) -> Option<Self::Item> {
        let child_bus_address = self.buff.take_by_cell_size(self.address_cell)?;

        let mut address = child_bus_address;

        if let Some(ranges) = &self.ranges {
            for one in ranges.iter() {
                let range_child_bus_address = one.child_bus_address().as_u64();
                let range_parent_bus_address = one.parent_bus_address().as_u64();

                if child_bus_address >= range_child_bus_address
                    && child_bus_address < range_child_bus_address + one.size
                {
                    address =
                        child_bus_address - range_child_bus_address + range_parent_bus_address;
                    break;
                }
            }
        }

        let size = if self.size_cell > 0 {
            Some(self.buff.take_by_cell_size(self.size_cell)? as usize)
        } else {
            None
        };
        Some(FdtReg {
            address,
            child_bus_address,
            size,
        })
    }
}

#[derive(Debug)]
pub enum NodeKind<'a> {
    General(Node<'a>),
    Chosen(Chosen<'a>),
    Memory(Memory<'a>),
}
