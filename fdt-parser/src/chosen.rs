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
        self.node.find_property("bootargs").map(|p| p.str())
    }

    /// Searches for the node representing `stdout`, if the property exists,
    /// attempting to resolve aliases if the node name doesn't exist as-is
    pub fn stdout(&self) -> Option<Stdout<'a>> {
        let path = self.node.find_property("stdout-path")?.str();
        let mut sp = path.split(':');
        let name = sp.next()?;
        let params = sp.next();
        let node = self.node.fdt.find_nodes(name).next()?;
        Some(Stdout { params, node })
    }

    pub fn debugcon(&self) -> Option<Node<'a>> {
        if let Some(node) = self.stdout() {
            Some(node.node)
        } else {
            fdt_bootargs_find_debugcon_node(self)
        }
    }
}

pub struct Stdout<'a> {
    pub params: Option<&'a str>,
    pub node: Node<'a>,
}

fn fdt_bootargs_find_debugcon_node<'a>(chosen: &Chosen<'a>) -> Option<Node<'a>> {
    let bootargs = chosen.bootargs()?;

    let earlycon = bootargs
        .split_ascii_whitespace()
        .find(|&arg| arg.contains("earlycon"))?;

    let mut tmp = earlycon.split('=');
    let _ = tmp.next()?;
    let values = tmp.next()?;

    let mut values = values.split(',');

    let name = values.next()?;

    if !name.contains("uart") {
        return None;
    }

    let param2 = values.next()?;
    let addr_str = if param2.contains("0x") {
        param2
    } else {
        values.next()?
    };

    let mmio = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16).ok()?;

    for node in chosen.node.fdt.all_nodes() {
        if let Some(regs) = node.reg() {
            for reg in regs {
                if reg.address.eq(&mmio) {
                    return Some(node);
                }
            }
        }
    }

    None
}
