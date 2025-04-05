use core::{iter, ptr::NonNull};

use crate::{
    chosen::Chosen, error::*, memory::Memory, meta::MetaData, node::Node, read::FdtReader,
    FdtHeader, MemoryRegion, Phandle, Token,
};

/// The reference to the FDT raw data.
#[derive(Clone)]
pub struct Fdt<'a> {
    pub(crate) header: FdtHeader,
    pub(crate) data: &'a [u8],
}

impl<'a> Fdt<'a> {
    /// Create a new FDT from raw data.
    pub fn from_bytes(data: &'a [u8]) -> FdtResult<'a, Self> {
        let header = FdtHeader::from_bytes(data)?;

        header.valid_magic()?;

        Ok(Self { header, data })
    }

    /// Create a new FDT from a pointer.
    pub fn from_ptr(ptr: NonNull<u8>) -> FdtResult<'a, Self> {
        let tmp_header =
            unsafe { core::slice::from_raw_parts(ptr.as_ptr(), core::mem::size_of::<FdtHeader>()) };
        let real_size = FdtHeader::from_bytes(tmp_header)?.totalsize.get() as usize;

        Self::from_bytes(unsafe { core::slice::from_raw_parts(ptr.as_ptr(), real_size) })
    }

    fn reader(&'a self, offset: usize) -> FdtReader<'a> {
        FdtReader::new(&self.data[offset..])
    }

    pub fn total_size(&self) -> usize {
        self.header.totalsize.get() as _
    }

    pub fn version(&self) -> usize {
        self.header.version.get() as _
    }

    /// This field shall contain the physical ID of the system’s boot CPU. It shall be identical to the physical ID given in the
    /// reg property of that CPU node within the devicetree.
    pub fn boot_cpuid_phys(&self) -> u32 {
        self.header.boot_cpuid_phys.get()
    }

    /// The memory reservation block provides the client program with a list of areas in physical memory which are reserved; that
    /// is, which shall not be used for general memory allocations. It is used to protect vital data structures from being overwritten
    /// by the client program.
    pub fn memory_reservation_block(&self) -> impl Iterator<Item = MemoryRegion> + '_ {
        let mut reader = self.reader(self.header.off_mem_rsvmap.get() as _);
        iter::from_fn(move || match reader.reserved_memory() {
            Some(region) => {
                if region.address == 0 && region.size == 0 {
                    None
                } else {
                    Some(region.into())
                }
            }
            None => None,
        })
    }

    /// Reserved memory is specified as a node under the `/reserved-memory` node. The operating system shall exclude reserved
    /// memory from normal usage. One can create child nodes describing particular reserved (excluded from normal use) memory
    /// regions. Such memory regions are usually designed for the special usage by various device drivers.
    pub fn reserved_memory(&self) -> impl Iterator<Item = Node<'a>> + 'a {
        self.find_nodes("/reserved-memory")
    }

    pub(crate) fn get_str(&self, offset: usize) -> FdtResult<'a, &'a str> {
        let string_bytes = &self.data[self.header.strings_range()];
        let reader = FdtReader::new(&string_bytes[offset..]);
        reader.peek_str()
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = Node<'a>> {
        self.new_fdt_itr()
    }

    fn new_fdt_itr(&self) -> FdtIter<'a> {
        let struct_bytes = &self.data[self.header.struct_range()];

        let reader = FdtReader::new(struct_bytes);
        FdtIter {
            fdt: self.clone(),
            current_level: 0,
            reader,
            stack: Default::default(),
            node_reader: None,
            node_name: "",
        }
    }

    pub fn chosen(&'a self) -> Option<Chosen<'a>> {
        self.find_nodes("/chosen").next().map(Chosen::new)
    }

    pub fn get_node_by_phandle(&self, phandle: Phandle) -> Option<Node<'a>> {
        self.all_nodes()
            .find(|x| match x.phandle() {
                Some(p) => p.eq(&phandle),
                None => false,
            })
            .clone()
    }

    pub fn get_node_by_name(&'a self, name: &str) -> Option<Node<'a>> {
        self.all_nodes().find(|x| x.name().eq(name)).clone()
    }

    pub fn find_compatible(&'a self, with: &'a [&'a str]) -> impl Iterator<Item = Node<'a>> + 'a {
        let mut all = self.all_nodes();

        iter::from_fn(move || loop {
            let node = all.next()?;
            let caps = node.compatibles();
            for cap in caps {
                for want in with {
                    if cap.eq(*want) {
                        return Some(node);
                    }
                }
            }
        })
    }

    /// if path start with '/' then search by path, else search by aliases
    pub fn find_nodes(&self, path: &'a str) -> impl Iterator<Item = Node<'a>> + 'a {
        let path = if path.starts_with("/") {
            path
        } else {
            self.find_aliase(path).expect("aliase not found")
        };

        IterFindNode::new(self.new_fdt_itr(), path)
    }

    pub fn find_aliase(&self, name: &str) -> Option<&'a str> {
        let aliases = self.find_nodes("/aliases").next()?;
        for prop in aliases.propertys() {
            if prop.name.eq(name) {
                return Some(prop.str());
            }
        }
        None
    }

    pub fn memory(&'a self) -> impl Iterator<Item = Memory<'a>> + 'a {
        self.find_nodes("/memory").map(Memory::new)
    }
}

pub struct FdtIter<'a> {
    fdt: Fdt<'a>,
    current_level: usize,
    reader: FdtReader<'a>,
    stack: [MetaData<'a>; 12],
    node_reader: Option<FdtReader<'a>>,
    node_name: &'a str,
}

impl<'a> FdtIter<'a> {
    fn get_meta_parent(&self) -> MetaData<'a> {
        let mut meta = MetaData::default();
        let level = match self.level_parent_index() {
            Some(l) => l,
            None => return MetaData::default(),
        } + 1;
        macro_rules! get_field {
            ($cell:ident) => {{
                let mut size = None;
                for i in (0..level).rev() {
                    if let Some(cell_size) = &self.stack[i].$cell {
                        size = Some(cell_size.clone());
                        break;
                    }
                }
                meta.$cell = size;
            }};
        }

        get_field!(address_cells);
        get_field!(size_cells);
        get_field!(clock_cells);
        get_field!(interrupt_cells);
        get_field!(gpio_cells);
        get_field!(dma_cells);
        get_field!(cooling_cells);
        get_field!(range);
        get_field!(interrupt_parent);

        meta
    }
    fn level_current_index(&self) -> usize {
        self.current_level - 1
    }
    fn level_parent_index(&self) -> Option<usize> {
        if self.level_current_index() > 0 {
            Some(self.level_current_index() - 1)
        } else {
            None
        }
    }

    fn handle_node_begin(&mut self) {
        self.current_level += 1;
        let i = self.level_current_index();
        self.stack[i] = MetaData::default();
        self.node_name = self.reader.take_unit_name().unwrap();
        self.node_reader = Some(self.reader.clone());
    }

    fn finish_node(&mut self) -> Option<Node<'a>> {
        let reader = self.node_reader.take()?;
        let level = self.current_level;
        let meta = self.stack[self.level_current_index()].clone();
        let meta_parent = self.get_meta_parent();

        let mut node = Node::new(&self.fdt, level, self.node_name, reader, meta_parent, meta);
        let ranges = node.node_ranges();
        self.stack[self.level_current_index()].range = ranges.clone();
        let ph = node.node_interrupt_parent();
        self.stack[self.level_current_index()].interrupt_parent = ph;

        node.meta = self.stack[self.level_current_index()].clone();

        Some(node)
    }
}

impl<'a> Iterator for FdtIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let token = self.reader.take_token()?;

            match token {
                Token::BeginNode => {
                    let node = self.finish_node();
                    self.handle_node_begin();
                    if node.is_some() {
                        return node;
                    }
                }
                Token::EndNode => {
                    let node = self.finish_node();
                    self.current_level -= 1;
                    if node.is_some() {
                        return node;
                    }
                }
                Token::Prop => {
                    let prop = self.reader.take_prop(&self.fdt)?;
                    let index = self.level_current_index();
                    macro_rules! update_cell {
                        ($cell:ident) => {
                            self.stack[index].$cell = Some(prop.u32() as _)
                        };
                    }
                    match prop.name {
                        "#address-cells" => update_cell!(address_cells),
                        "#size-cells" => update_cell!(size_cells),
                        "#clock-cells" => update_cell!(clock_cells),
                        "#interrupt-cells" => update_cell!(interrupt_cells),
                        "#gpio-cells" => update_cell!(gpio_cells),
                        "#dma-cells" => update_cell!(dma_cells),
                        "#cooling-cells" => update_cell!(cooling_cells),
                        _ => {}
                    }
                }
                Token::End => {
                    return self.finish_node();
                }
                _ => {}
            }
        }
    }
}

struct IterFindNode<'a> {
    itr: FdtIter<'a>,
    want: &'a str,
    want_itr: usize,
    is_path_last: bool,
}

impl<'a> IterFindNode<'a> {
    fn new(itr: FdtIter<'a>, want: &'a str) -> Self {
        IterFindNode {
            itr,
            want,
            want_itr: 0,
            is_path_last: false,
        }
    }
}

impl<'a> Iterator for IterFindNode<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut out = None;
        loop {
            let mut parts = self.want.split("/").filter(|o| !o.is_empty());
            let mut want_part = "/";
            for _ in 0..self.want_itr {
                if let Some(part) = parts.next() {
                    want_part = part;
                } else {
                    self.is_path_last = true;
                    if let Some(out) = out {
                        return Some(out);
                    }
                }
            }
            let node = self.itr.next()?;

            let eq = if want_part.contains("@") {
                node.name.eq(want_part)
            } else {
                let name = node.name.split("@").next().unwrap();
                name.eq(want_part)
            };
            if eq {
                self.want_itr += 1;
                out = Some(node);
            }
        }
    }
}
