use alloc::{string::String, vec::Vec};

use crate::Property;

#[derive(Clone)]
pub struct Node {
    pub name: String,
    pub properties: Vec<Property>,
    pub children: Vec<Node>,
}
