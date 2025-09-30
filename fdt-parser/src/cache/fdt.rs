use core::ops::Deref;

use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    string::String,
    sync::Arc,
    vec::Vec,
};

use super::{Align4Vec, Node};
use crate::{base, cache::NodeMeta, data::Raw, FdtError, Header, MemoryRegion, Phandle};

#[derive(Clone)]
pub struct Fdt {
    inner: Arc<Inner>,
}

impl Fdt {
    /// Create a new `Fdt` from byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Fdt, FdtError> {
        let b = base::Fdt::from_bytes(data)?;
        let mut inner = Inner {
            raw: Align4Vec::new(data),
            phandle_cache: BTreeMap::new(),
            compatible_cache: BTreeMap::new(),
            name_cache: BTreeMap::new(),
        };
        let mut node_vec = Vec::new();
        for node in b.all_nodes() {
            let node = node?;
            inner
                .name_cache
                .entry(node.name().into())
                .or_insert_with(|| NodeMeta::new(&node));

            if let Some(phandle) = node.phandle()? {
                inner
                    .phandle_cache
                    .entry(phandle)
                    .or_insert_with(|| node.name().into());
            }
            for compatible in node.compatibles_flatten() {
                let map = inner.compatible_cache.entry(compatible.into()).or_default();
                map.insert(node.name().into());
            }
            node_vec.push(node);
        }

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

    pub fn header(&self) -> Header {
        self.fdt_base().header().clone()
    }

    /// With caching
    pub fn all_nodes(&self) -> Vec<Node> {
        self.inner
            .name_cache
            .values()
            .map(|meta| Node::new(self, meta))
            .collect()
    }

    /// if path start with '/' then search by path, else search by aliases
    pub fn find_nodes(&self, path: impl AsRef<str>) -> Vec<Node> {
        let fdt = self.fdt_base();
        let mut out = Vec::new();
        for n in fdt.find_nodes(path.as_ref()).flatten() {
            out.push(Node::new(self, &NodeMeta::new(&n)));
        }
        out
    }

    pub fn find_aliase(&self, name: impl AsRef<str>) -> Option<String> {
        let fdt = self.fdt_base();
        let s = fdt.find_aliase(name.as_ref()).ok()?;
        Some(s.into())
    }

    pub fn get_node_by_phandle(&self, phandle: Phandle) -> Option<Node> {
        let name = self.inner.phandle_cache.get(&phandle)?;
        let meta = self.inner.name_cache.get(name)?;
        Some(Node::new(self, meta))
    }

    pub fn find_compatible(&self, with: &[&str]) -> Vec<Node> {
        let mut names = BTreeSet::new();
        for &c in with {
            if let Some(s) = self.inner.compatible_cache.get(c) {
                for n in s {
                    names.insert(n);
                }
            }
        }
        let mut out = Vec::new();
        for name in names {
            if let Some(meta) = self.inner.name_cache.get(name) {
                out.push(Node::new(self, meta));
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

struct Inner {
    raw: Align4Vec,
    phandle_cache: BTreeMap<Phandle, String>,
    /// compatible -> set(name)
    compatible_cache: BTreeMap<String, BTreeSet<String>>,
    name_cache: BTreeMap<String, NodeMeta>,
}

unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}
