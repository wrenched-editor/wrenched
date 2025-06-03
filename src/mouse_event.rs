#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Click {
    Single,
    Double,
    Tripple,
}

impl Click {
    pub fn from_count(count: u32) -> Click {
        match count {
            2 => Click::Double,
            3 => Click::Tripple,
            _ => Click::Single,
        }
    }
}
