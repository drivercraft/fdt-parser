use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{fdt_no_mem::FdtNoMem, FdtError, Header, MemoryRegion, Node, Phandle};

#[derive(Clone)]
pub struct Fdt<'a> {
    inner: FdtNoMem<'a>,
    phandle_cache: BTreeMap<Phandle, Node<'a>>,
    /// compatible -> (name -> node)
    compatible_cache: BTreeMap<&'a str, BTreeMap<&'a str, Node<'a>>>,
    name_cache: BTreeMap<&'a str, Node<'a>>,
}

impl<'a> Fdt<'a> {
    /// Create a new `Fdt` from byte slice.
    pub fn from_bytes(data: &'a [u8]) -> Result<Fdt<'a>, FdtError> {
        let inner = FdtNoMem::from_bytes(data)?;
        Ok(Fdt {
            inner,
            phandle_cache: BTreeMap::new(),
            compatible_cache: BTreeMap::new(),
            name_cache: BTreeMap::new(),
        })
    }

    /// Create a new `Fdt` from a raw pointer and size in bytes.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a
    /// memory region of at least `size` bytes that contains a valid device tree
    /// blob.
    pub unsafe fn from_ptr(ptr: *mut u8) -> Result<Fdt<'a>, FdtError> {
        let inner = FdtNoMem::from_ptr(ptr)?;
        Ok(Fdt {
            inner,
            phandle_cache: BTreeMap::new(),
            compatible_cache: BTreeMap::new(),
            name_cache: BTreeMap::new(),
        })
    }

    /// Get a reference to the FDT header.
    pub fn header(&self) -> &Header {
        self.inner.header()
    }

    pub fn total_size(&self) -> usize {
        self.inner.total_size()
    }

    /// This field shall contain the physical ID of the system's boot CPU. It shall be identical to the physical ID given in the
    /// reg property of that CPU node within the devicetree.
    pub fn boot_cpuid_phys(&self) -> u32 {
        self.inner.boot_cpuid_phys()
    }

    /// Get a reference to the underlying buffer.
    pub fn raw(&self) -> &'a [u8] {
        self.inner.raw()
    }

    /// Get the FDT version
    pub fn version(&self) -> u32 {
        self.inner.version()
    }

    pub fn memory_reservaion_blocks(&self) -> impl Iterator<Item = MemoryRegion> + 'a {
        self.inner.memory_reservaion_blocks()
    }

    /// With caching
    pub fn all_nodes(&mut self) -> Result<Vec<Node<'a>>, FdtError> {
        let nodes = self.inner.all_nodes().collect::<Result<Vec<_>, _>>()?;
        for node in &nodes {
            if let Some(phandle) = node.phandle()? {
                self.phandle_cache
                    .entry(phandle)
                    .or_insert_with(|| node.clone());
            }
            for compatibles in node.compatibles_flatten() {
                let map = self.compatible_cache.entry(compatibles).or_default();
                map.entry(node.name()).or_insert_with(|| node.clone());
            }
            self.name_cache
                .entry(node.name())
                .or_insert_with(|| node.clone());
        }
        Ok(nodes)
    }

    /// if path start with '/' then search by path, else search by aliases
    pub fn find_nodes(&self, path: &'a str) -> Result<Vec<Node<'a>>, FdtError> {
        self.inner.find_nodes(path).collect()
    }

    pub fn find_aliase(&self, name: &str) -> Result<&'a str, FdtError> {
        self.inner.find_aliase(name)
    }

    pub fn get_node_by_phandle(&self, phandle: Phandle) -> Result<Option<Node<'a>>, FdtError> {
        if let Some(node) = self.phandle_cache.get(&phandle) {
            return Ok(Some(node.clone()));
        }
        self.inner.get_node_by_phandle(phandle)
    }

    pub fn find_compatible(&self, with: &[&str]) -> Result<Vec<Node<'a>>, FdtError> {
        if self.compatible_cache.is_empty() {
            self.inner.find_compatible(with).collect()
        } else {
            let mut result = Vec::new();
            for &c in with {
                if let Some(map) = self.compatible_cache.get(c) {
                    for node in map.values() {
                        result.push(node.clone());
                    }
                }
            }
            Ok(result)
        }
    }
}
