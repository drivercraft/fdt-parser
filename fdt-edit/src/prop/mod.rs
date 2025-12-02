use alloc::{string::String, vec::Vec};

#[derive(Clone)]
pub struct Property {
    pub name: String,
    pub kind: PropertyKind,
}

#[derive(Clone)]
pub enum PropertyKind {
    AddressCells(u8),
    SizeCells(u8),
    Unknown(RawProperty),
}

#[derive(Clone)]
pub struct RawProperty {
    pub data: Vec<u8>,
}
