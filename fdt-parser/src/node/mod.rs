use crate::{data::Raw, Fdt};

pub struct Node<'a> {
    pub(crate) fdt: Fdt<'a>,
    pub(crate) name: &'a str,
    pub(crate) level: usize,
    pub(crate) pos: usize,
}

impl<'a> Node<'a> {
    pub(crate) fn new(fdt: &Fdt<'a>, name: &'a str, level: usize, pos: usize) -> Self {
        Node {
            fdt: fdt.clone(),
            name,
            level,
            pos,
        }
    }

    fn raw(&self) -> Raw<'a> {
        self.fdt.raw.begin_at(self.pos).unwrap()
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
    pub fn compatible(&self) -> Option<impl Iterator<Item = &'a str>> {
        // This is a placeholder - would need to parse properties
        None::<core::iter::Empty<&str>>
    }

    /// Get register values for this node (placeholder implementation)
    pub fn reg(&self) -> Option<impl Iterator<Item = u64>> {
        // This is a placeholder - would need to parse properties
        None::<core::iter::Empty<u64>>
    }

    pub fn kind(&self) -> NodeKind {
        NodeKind::General
    }
}

#[derive(Debug)]
pub enum NodeKind {
    General,
}
