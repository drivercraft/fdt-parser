use crate::data::{Bytes, Reader};

#[derive(Clone)]
pub struct VecRange<'a> {
    address_cells: usize,
    parent_address_cells: usize,
    size_cells: usize,
    data: Bytes<'a>,
}

impl<'a> VecRange<'a> {
    pub(crate) fn new(
        address_cells: usize,
        parent_address_cells: usize,
        size_cells: usize,
        data: Bytes<'a>,
    ) -> Self {
        Self {
            address_cells,
            parent_address_cells,
            size_cells,
            data,
        }
    }

    pub fn iter(&self) -> VecRangeIter<'a> {
        VecRangeIter {
            address_cells: self.address_cells,
            parent_address_cells: self.parent_address_cells,
            size_cells: self.size_cells,
            reader: self.data.reader(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RangeInfo {
    pub child_address: u64,
    pub parent_address: u64,
    pub length: u64,
}

pub struct VecRangeIter<'a> {
    address_cells: usize,
    parent_address_cells: usize,
    size_cells: usize,
    reader: Reader<'a>,
}

impl<'a> Iterator for VecRangeIter<'a> {
    type Item = RangeInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let child_address = self.reader.read_cells(self.address_cells)?;
        let parent_address = self.reader.read_cells(self.parent_address_cells)?;
        let length = self.reader.read_cells(self.size_cells)?;

        Some(RangeInfo {
            child_address,
            parent_address,
            length,
        })
    }
}
