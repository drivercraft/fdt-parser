pub const FDT_MAGIC: u32 = 0xd00dfeed;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Token {
    BeginNode,
    EndNode,
    Prop,
    Nop,
    End,
    Data,
}

impl From<u32> for Token {
    fn from(value: u32) -> Self {
        match value {
            0x1 => Token::BeginNode,
            0x2 => Token::EndNode,
            0x3 => Token::Prop,
            0x4 => Token::Nop,
            0x9 => Token::End,
            _ => Token::Data,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Okay,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct ReserveEntry {
    pub address: u64,
    pub size: u64,
}
