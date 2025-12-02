use crate::{Fdt, FdtError, Node, NodeIter, Token, data::Reader};

pub struct FdtIter<'a> {
    fdt: Fdt<'a>,
    reader: Reader<'a>,
    node_iter: Option<NodeIter<'a>>,
    has_err: bool,
}

impl<'a> FdtIter<'a> {
    pub fn new(fdt: Fdt<'a>) -> Self {
        let reader = fdt.data.reader();
        Self {
            fdt,
            reader,
            node_iter: None,
            has_err: false,
        }
    }
}

impl<'a> Iterator for FdtIter<'a> {
    type Item = Result<Node<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_err {
            return None;
        }
        loop {
            if let Some(ref mut node_iter) = self.node_iter {
                if let Some(node_res) = node_iter.next() {
                    self.has_err = node_res.is_err();
                    return Some(node_res);
                } else {
                    let len = node_iter.taken_len();
                    self.has_err = self.reader.skip_align(len).is_err();
                    self.node_iter = None;
                }
            }

            match self.reader.read_token() {
                Ok(Token::BeginNode) => {
                    let node_iter = NodeIter::new(self.reader.clone());
                    self.node_iter = Some(node_iter);
                }
                Ok(Token::End) => {
                    return None;
                }
                Err(e) => {
                    self.has_err = true;
                    return Some(Err(e));
                }
                _ => {}
            }
        }
    }
}
