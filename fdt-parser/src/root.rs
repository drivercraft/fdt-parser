use core::iter;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    data::{Buffer, Raw},
    fdt_no_mem::FdtNoMem,
    node::NodeBase,
    Chosen, FdtError, Header, Memory, MemoryRegion, Node, Phandle, Token,
};

#[derive(Clone)]
pub struct Fdt<'a> {
    inner: FdtNoMem<'a>,
    phandle_cache: BTreeMap<Phandle, Node<'a>>,
}

impl<'a> Fdt<'a> {
    /// Create a new `Fdt` from byte slice.
    pub fn from_bytes(data: &'a [u8]) -> Result<Fdt<'a>, FdtError> {
        let inner = FdtNoMem::from_bytes(data)?;
        Ok(Fdt {
            inner,
            phandle_cache: BTreeMap::new(),
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

    pub(crate) fn get_str(&self, offset: usize) -> Result<&'a str, FdtError> {
        self.inner.get_str(offset)
    }

    pub fn all_nodes(&mut self) -> Result<Vec<Node<'a>>, FdtError> {
        let nodes = self.inner.all_nodes().collect::<Result<Vec<_>, _>>()?;
        for node in &nodes {
            if let Some(phandle) = node.phandle()? {
                if !self.phandle_cache.contains_key(&phandle) {
                    self.phandle_cache.insert(phandle, node.clone());
                }
            }
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
}
