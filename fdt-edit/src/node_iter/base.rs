use core::fmt::Display;

use crate::Node;

pub struct NodeRef<'a> {
    pub(crate) meta: NodeIterMeta,
    pub(crate) node: &'a Node,
}

pub struct NodeRefMut<'a> {
    pub(crate) meta: NodeIterMeta,
    pub(crate) node: &'a mut Node,
}

#[derive(Clone)]
pub(crate) struct NodeIterMeta {
    pub(crate) level: usize,
    pub(crate) address_cells: usize,
    pub(crate) size_cells: usize,
}

impl NodeIterMeta {
    fn write_indent(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for _ in 0..self.level {
            write!(f, "  ")?; // Indent based on level
        }
        Ok(())
    }
}

impl Display for NodeRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.meta.write_indent(f)?;

        writeln!(f, "{}", self.node.name)?;
        for prop in self.node.properties() {
            self.meta.write_indent(f)?;
            write!(f, "  {} = ", prop.name())?;

            if let Some(str) = prop.as_str() {
                writeln!(f, "\"{}\";", str)?;
            } else {
                for cell in prop.get_u32_iter() {
                    write!(f, "{:#x} ", cell)?;
                }
                writeln!(f, ";")?;
            }
        }

        Ok(())
    }
}
