use crate::{
    FdtError,
    data::{Bytes, Reader},
};

#[derive(Clone)]
pub struct Node<'a> {
    name: &'a str,
    data: Bytes<'a>,
}

impl<'a> Node<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn data(&self) -> &Bytes<'a> {
        &self.data
    }
}

pub(crate) struct NodeIter<'a> {
    reader: Reader<'a>,
}

impl<'a> NodeIter<'a> {
    pub fn new(reader: Reader<'a>) -> Self {
        Self { reader }
    }

    pub fn taken_len(&self) -> usize {
        self.reader.taken_len()
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = Result<Node<'a>, FdtError>;
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
