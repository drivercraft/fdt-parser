use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use super::{Align4Vec, Node};
use crate::{base, cache::NodeMeta, data::Raw, FdtError, Header, MemoryRegion, Phandle};

#[derive(Clone)]
pub struct Fdt {
    pub(super) inner: Arc<Inner>,
}

impl Fdt {
    /// Create a new `Fdt` from byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Fdt, FdtError> {
        let inner = Inner::new(data)?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Create a new `Fdt` from a raw pointer and size in bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// memory region of at least `size` bytes that contains a valid device tree
    /// blob.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Fdt, FdtError> {
        let b = base::Fdt::from_ptr(ptr)?;
        Self::from_bytes(b.raw())
    }

    pub(super) fn fdt_base<'a>(&'a self) -> base::Fdt<'a> {
        base::Fdt::from_bytes(&self.inner.raw).unwrap()
    }

    pub fn version(&self) -> u32 {
        self.fdt_base().version()
    }

    pub fn header(&self) -> Header {
        self.fdt_base().header().clone()
    }

    pub fn all_nodes(&self) -> Vec<Node> {
        self.inner
            .all_nodes
            .iter()
            .map(|meta| Node::new(self, meta))
            .collect()
    }

    /// if path start with '/' then search by path, else search by aliases
    pub fn find_nodes(&self, path: impl AsRef<str>) -> Vec<Node> {
        let path = path.as_ref();
        let path = if path.starts_with("/") {
            path.to_string()
        } else {
            self.find_aliase(path).unwrap()
        };
        let mut out = Vec::new();
        for node in self.all_nodes() {
            if node.full_path().starts_with(path.as_str()) {
                let right = node.full_path().trim_start_matches(&path);
                if right.split("/").count() < 2 {
                    out.push(node);
                }
            }
        }

        out
    }

    pub fn find_aliase(&self, name: impl AsRef<str>) -> Option<String> {
        let fdt = self.fdt_base();
        let s = fdt.find_aliase(name.as_ref()).ok()?;
        Some(s.into())
    }

    pub fn get_node_by_phandle(&self, phandle: Phandle) -> Option<Node> {
        let meta = self.inner.get_node_by_phandle(phandle)?;
        Some(Node::new(self, &meta))
    }

    pub fn find_compatible(&self, with: &[&str]) -> Vec<Node> {
        let mut ids = BTreeSet::new();
        for &c in with {
            if let Some(s) = self.inner.compatible_cache.get(c) {
                for n in s {
                    ids.insert(n);
                }
            }
        }
        let mut out = Vec::new();
        for id in ids {
            if let Some(meta) = self.inner.get_node_by_index(*id) {
                out.push(Node::new(self, &meta));
            }
        }

        out
    }

    pub fn memory_reservaion_blocks(&self) -> Vec<MemoryRegion> {
        let fdt = self.fdt_base();
        fdt.memory_reservaion_blocks().collect()
    }

    pub fn raw<'a>(&'a self) -> Raw<'a> {
        Raw::new(&self.inner.raw)
    }
}

pub(super) struct Inner {
    raw: Align4Vec,
    phandle_cache: BTreeMap<Phandle, usize>,
    /// compatible -> set(name)
    compatible_cache: BTreeMap<String, BTreeSet<usize>>,
    /// same order as all_nodes()
    all_nodes: Vec<NodeMeta>,
    path_cache: BTreeMap<String, usize>,
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

impl Inner {
    fn new(data: &[u8]) -> Result<Self, FdtError> {
        let b = base::Fdt::from_bytes(data)?;
        let mut inner = Inner {
            raw: Align4Vec::new(data),
            phandle_cache: BTreeMap::new(),
            compatible_cache: BTreeMap::new(),
            all_nodes: Vec::new(),
            path_cache: BTreeMap::new(),
        };
        let mut node_vec = Vec::new();
        let mut path_stack = Vec::new();
        let mut node_stack: Vec<NodeMeta> = Vec::new();
        for (i, node) in b.all_nodes().enumerate() {
            let node = node?;
            let node_name = node.name();
            let level = node.level();

            while let Some(last) = node_stack.last() {
                if level <= last.level {
                    node_stack.pop();
                } else {
                    break;
                }
            }

            if level < path_stack.len() {
                path_stack.truncate(level);
            }
            path_stack.push(node_name.trim_start_matches("/"));
            let full_path = if path_stack.len() > 1 {
                alloc::format!("/{}", path_stack[1..].join("/"))
            } else {
                "/".to_string()
            };
            for prop in node.properties() {
                let _ = prop?;
            }
            let parent = node_stack.last();
            let dnode = NodeMeta::new(&node, full_path.clone(), parent);
            node_stack.push(dnode.clone());
            inner.all_nodes.push(dnode.clone());
            inner.path_cache.insert(full_path, i);

            if let Some(phandle) = node.phandle()? {
                inner.phandle_cache.entry(phandle).or_insert_with(|| i);
            }
            for compatible in node.compatibles_flatten() {
                let map = inner.compatible_cache.entry(compatible.into()).or_default();
                map.insert(i);
            }
            node_vec.push(node);
        }

        Ok(inner)
    }

    pub(crate) fn get_node_by_path(&self, path: &str) -> Option<NodeMeta> {
        let idx = self.path_cache.get(path)?;
        Some(self.all_nodes[*idx].clone())
    }

    fn get_node_by_index(&self, index: usize) -> Option<NodeMeta> {
        self.all_nodes.get(index).cloned()
    }

    fn get_node_by_phandle(&self, phandle: Phandle) -> Option<NodeMeta> {
        let idx = self.phandle_cache.get(&phandle)?;
        Some(self.all_nodes[*idx].clone())
    }
}
