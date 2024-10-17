use crate::{node::Node, FdtRangeSilce, Phandle};

#[derive(Clone, Default)]
pub(crate) struct MetaData<'a> {
    pub address_cells: Option<u8>,
    pub size_cells: Option<u8>,
    pub clock_cells: Option<u8>,
    pub interrupt_cells: Option<u8>,
    pub gpio_cells: Option<u8>,
    pub dma_cells: Option<u8>,
    pub cooling_cells: Option<u8>,
    pub range: Option<FdtRangeSilce<'a>>,
    pub interrupt_parent: Option<Phandle>,
}
