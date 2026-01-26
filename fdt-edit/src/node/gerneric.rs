use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{fmt::Debug, ops::Deref};
use fdt_raw::RegInfo;

use crate::{Context, Node, NodeMut, Property};

/// Generic node reference with context.
///
/// Provides basic node access operations with context-aware functionality
/// for traversing and manipulating device tree nodes.
#[derive(Clone)]
pub struct NodeRefGen<'a> {
    /// The underlying node reference
    pub node: &'a Node,
    /// The parsing context containing parent information and path
    pub ctx: Context<'a>,
}

impl<'a> NodeRefGen<'a> {
    pub fn find_property(&self, name: &str) -> Option<&'a Property> {
        self.node.get_property(name)
    }

    pub fn properties(&self) -> impl Iterator<Item = &'a Property> {
        self.node.properties.iter()
    }

    fn op(&'a self) -> RefOp<'a> {
        RefOp {
            ctx: &self.ctx,
            node: self.node,
        }
    }

    pub fn path(&self) -> String {
        self.op().path()
    }

    pub fn path_eq(&self, path: &str) -> bool {
        self.op().ref_path_eq(path)
    }

    pub fn path_eq_fuzzy(&self, path: &str) -> bool {
        self.op().ref_path_eq_fuzzy(path)
    }

    pub fn regs(&self) -> Option<Vec<RegFixed>> {
        self.op().regs()
    }
}

impl Deref for NodeRefGen<'_> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node
    }
}

/// Generic mutable node reference with context.
///
/// Provides mutable node operations with context-aware functionality
/// for modifying device tree nodes and their properties.
pub struct NodeMutGen<'a> {
    /// The underlying mutable node reference
    pub node: &'a mut Node,
    /// The parsing context containing parent information and path
    pub ctx: Context<'a>,
}

impl<'a> NodeMutGen<'a> {
    fn op(&'a self) -> RefOp<'a> {
        RefOp {
            ctx: &self.ctx,
            node: self.node,
        }
    }

    pub fn path(&self) -> String {
        self.op().path()
    }

    pub fn path_eq(&self, path: &str) -> bool {
        self.op().ref_path_eq(path)
    }

    pub fn path_eq_fuzzy(&self, path: &str) -> bool {
        self.op().ref_path_eq_fuzzy(path)
    }

    pub fn regs(&self) -> Option<Vec<RegFixed>> {
        self.op().regs()
    }

    /// Sets the reg property with automatic address translation.
    ///
    /// This method converts CPU physical addresses to bus addresses using the
    /// parent node's ranges mapping before storing them in the reg property.
    pub fn set_regs(&mut self, regs: &[RegInfo]) {
        let address_cells = self.ctx.parent_address_cells() as usize;
        let size_cells = self.ctx.parent_size_cells() as usize;
        let ranges = self.ctx.current_ranges();

        let mut data = Vec::new();

        for reg in regs {
            // Convert CPU address to bus address
            let mut bus_address = reg.address;
            if let Some(ref ranges) = ranges {
                for r in ranges {
                    // Check if CPU address is within ranges mapping range
                    if reg.address >= r.parent_bus_address
                        && reg.address < r.parent_bus_address + r.length
                    {
                        // Reverse conversion: cpu_address -> bus_address
                        bus_address = reg.address - r.parent_bus_address + r.child_bus_address;
                        break;
                    }
                }
            }

            // Write bus address (big-endian)
            if address_cells == 1 {
                data.extend_from_slice(&(bus_address as u32).to_be_bytes());
            } else if address_cells == 2 {
                data.extend_from_slice(&((bus_address >> 32) as u32).to_be_bytes());
                data.extend_from_slice(&((bus_address & 0xFFFF_FFFF) as u32).to_be_bytes());
            }

            // Write size (big-endian)
            if size_cells == 1 {
                let size = reg.size.unwrap_or(0);
                data.extend_from_slice(&(size as u32).to_be_bytes());
            } else if size_cells == 2 {
                let size = reg.size.unwrap_or(0);
                data.extend_from_slice(&((size >> 32) as u32).to_be_bytes());
                data.extend_from_slice(&((size & 0xFFFF_FFFF) as u32).to_be_bytes());
            }
        }

        let prop = Property::new("reg", data);
        self.node.set_property(prop);
    }

    /// Adds a child node to this node.
    ///
    /// This method attaches a child node to the current node, updating the
    /// context to include the parent-child relationship, and returns a
    /// mutable reference to the newly added child.
    pub fn add_child(&mut self, child: Node) -> NodeMut<'a> {
        let name = child.name().to_string();
        let mut ctx = self.ctx.clone();
        unsafe {
            let node_ptr = self.node as *mut Node;
            let node = &*node_ptr;
            ctx.push(node);
        }
        self.node.add_child(child);
        let raw = self.node.get_child_mut(&name).unwrap();
        unsafe {
            let node_ptr = raw as *mut Node;
            let node = &mut *node_ptr;
            NodeMut::new(node, ctx)
        }
    }
}

impl Debug for NodeRefGen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NodeRefGen {{ name: {} }}", self.node.name())
    }
}

impl Debug for NodeMutGen<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NodeMutGen {{ name: {} }}", self.node.name())
    }
}

/// Internal helper struct for node operations with context.
///
/// This struct provides common operations that are shared between
/// `NodeRefGen` and `NodeMutGen`, avoiding code duplication.
struct RefOp<'a> {
    /// Reference to the parsing context
    ctx: &'a Context<'a>,
    /// Reference to the node being operated on
    node: &'a Node,
}

impl<'a> RefOp<'a> {
    /// Constructs the full path of the node.
    ///
    /// Combines the current context path with the node name to create
    /// the full device tree path.
    fn path(&self) -> String {
        self.ctx.current_path() + "/" + self.node.name()
    }

    /// Checks if the node's path exactly matches the given path.
    fn ref_path_eq(&self, path: &str) -> bool {
        self.path() == path
    }

    /// Checks if the node's path matches the given path using fuzzy matching.
    ///
    /// Fuzzy matching allows comparing paths without requiring the exact
    /// address portion (the `@address` suffix) to match unless explicitly
    /// specified. This is useful for matching nodes by name when the
    /// specific address is not important.
    fn ref_path_eq_fuzzy(&self, path: &str) -> bool {
        let mut want = path.trim_matches('/').split("/");
        let got_path = self.path();
        let mut got = got_path.trim_matches('/').split("/");
        let got_count = got.clone().count();
        let mut current = 0;

        loop {
            let w = want.next();
            let g = got.next();
            let is_last = current + 1 == got_count;

            match (w, g) {
                (Some(w), Some(g)) => {
                    if w != g && !is_last {
                        return false;
                    }

                    let name = g.split('@').next().unwrap_or(g);
                    let addr = g.split('@').nth(1);

                    let want_name = w.split('@').next().unwrap_or(w);
                    let want_addr = w.split('@').nth(1);

                    let res = match (addr, want_addr) {
                        (Some(a), Some(wa)) => name == want_name && a == wa,
                        (Some(_), None) => name == want_name,
                        (None, Some(_)) => false,
                        (None, None) => name == want_name,
                    };
                    if !res {
                        return false;
                    }
                }
                (None, _) => break,
                _ => return false,
            }
            current += 1;
        }
        true
    }

    /// Parses the reg property and returns a list of register regions.
    ///
    /// This method reads the reg property and performs address translation
    /// from child bus addresses to CPU physical addresses using the parent's
    /// ranges mapping.
    fn regs(&self) -> Option<Vec<RegFixed>> {
        let prop = self.node.get_property("reg")?;
        let mut iter = prop.as_reader();
        let address_cells = self.ctx.parent_address_cells() as usize;
        let size_cells = self.ctx.parent_size_cells() as usize;

        // Get current ranges from context
        let ranges = self.ctx.current_ranges();
        let mut out = vec![];
        let mut size;

        while let Some(mut address) = iter.read_cells(address_cells) {
            if size_cells > 0 {
                size = iter.read_cells(size_cells);
            } else {
                size = None;
            }
            let child_bus_address = address;

            if let Some(ref ranges) = ranges {
                for r in ranges {
                    if child_bus_address >= r.child_bus_address
                        && child_bus_address < r.child_bus_address + r.length
                    {
                        address = child_bus_address - r.child_bus_address + r.parent_bus_address;
                        break;
                    }
                }
            }

            let reg = RegFixed {
                address,
                child_bus_address,
                size,
            };
            out.push(reg);
        }

        Some(out)
    }
}

/// Fixed register region with address translation information.
///
/// Represents a single register region from the reg property with both
/// the bus address (stored in the DTB) and the translated CPU physical address.
#[derive(Clone, Copy, Debug)]
pub struct RegFixed {
    /// CPU physical address after translation
    pub address: u64,
    /// Child bus address as stored in the reg property
    pub child_bus_address: u64,
    /// Size of the register region (None if size-cells is 0)
    pub size: Option<u64>,
}
