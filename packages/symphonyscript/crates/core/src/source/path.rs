use alloc::vec::Vec;
use core::fmt;

use super::segment::Segment;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Path {
    pub segments: Vec<Segment>,
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (position, segment) in self.segments.iter().enumerate() {
            if position > 0 {
                f.write_str(".")?;
            }
            write!(f, "{segment}")?;
        }
        Ok(())
    }
}
