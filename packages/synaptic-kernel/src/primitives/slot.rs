use std::fmt;
use std::fmt::Formatter;
use std::num::NonZeroU32;

/// Typed identifier for slots.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotId(NonZeroU32);

impl SlotId {
    #[inline]
    pub fn new(value: u32) -> Option<SlotId> {
        NonZeroU32::new(value).map(SlotId)
    }

    #[inline]
    pub fn from_i32(value: i32) -> Option<SlotId> {
        if value <= 0 {
            None
        } else {
            Self::new(value as u32)
        }
    }

    #[inline]
    pub fn option_to_i32(slot: Option<SlotId>) -> i32 {
        match slot {
            Some(s) => s.get() as i32,
            None => 0,
        }
    }

    #[inline]
    pub fn get(&self) -> u32 {
        self.0.get()
    }

    #[inline]
    pub fn to_i32(&self) -> i32 {
        self.0.get() as i32
    }

    #[inline]
    pub fn to_usize(&self) -> usize {
        self.0.get() as usize
    }
}

impl fmt::Display for SlotId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SlotId> for usize {
    #[inline]
    fn from(slot: SlotId) -> usize {
        slot.0.get() as usize
    }
}
