use core::fmt::Display;

use crate::Node;

pub struct NodeGeneric {
    pub(crate) meta: NodeIterMeta,
    pub(crate) node: *mut Node,
}

impl NodeGeneric {
    pub(crate) fn new(node: *mut Node, meta: NodeIterMeta) -> Self {
        Self { node, meta }
    }

    pub fn as_node<'a>(&self) -> &'a Node {
        unsafe { &*self.node }
    }

    pub fn as_node_mut<'a>(&mut self) -> &'a mut Node {
        unsafe { &mut *self.node }
    }
}

unsafe impl Send for NodeGeneric {}

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

impl Display for NodeGeneric {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.meta.write_indent(f)?;

        writeln!(f, "{}", self.as_node().name)?;
        for prop in self.as_node().properties() {
            self.meta.write_indent(f)?;
            write!(f, "  {} = ", prop.name())?;

            if prop.name() == "compatible" {
                write!(f, "[")?;
                for (i, str) in prop.as_str_iter().enumerate() {
                    write!(f, "\"{}\"", str)?;
                    if i != prop.as_str_iter().count() - 1 {
                        write!(f, ", ")?;
                    }
                }
                writeln!(f, "]")?;
                continue;
            }

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
