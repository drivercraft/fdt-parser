use crate::node::Node;

pub struct Chosen<'a> {
    node: Node<'a>,
}

impl<'a> Chosen<'a> {
    pub fn new(node: Node<'a>) -> Self {
        Chosen { node }
    }

    /// Contains the bootargs, if they exist
    pub fn bootargs(&self) -> Option<&'a str> {
        self.node
            .find_property("bootargs")
            .and_then(|p| Some(p.str()))
    }

    /// Searches for the node representing `stdout`, if the property exists,
    /// attempting to resolve aliases if the node name doesn't exist as-is
    pub fn stdout(&self) -> Option<Stdout<'a>> {
        let path = self.node.find_property("stdout-path")?.str();
        let mut sp = path.split(':');
        let name = sp.next()?;
        let params = sp.next();
        let node = self.node.fdt.find_node(name)?;
        return Some(Stdout { params, node });
    }
}

pub struct Stdout<'a> {
    pub params: Option<&'a str>,
    pub node: Node<'a>,
}
