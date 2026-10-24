use alloc::string::String;
use core::fmt;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Segment {
    Named(String),
    Anonymous { name: String, index: u32 },
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Segment::Named(name) => f.write_str(name),
            Segment::Anonymous { name, index } => write!(f, "{name}:{index}"),
        }
    }
}
