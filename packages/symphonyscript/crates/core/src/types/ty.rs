use alloc::vec::Vec;

use super::scalar::Scalar;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Ty {
    pub scalar: Scalar,
    pub lengths: Vec<u32>,
}

impl Ty {
    pub const I32: Ty = Ty::of(Scalar::I32);
    pub const I64: Ty = Ty::of(Scalar::I64);
    pub const F32: Ty = Ty::of(Scalar::F32);
    pub const F64: Ty = Ty::of(Scalar::F64);
    pub const BOOL: Ty = Ty::of(Scalar::Bool);
    pub const STR: Ty = Ty::of(Scalar::Str);

    pub const fn of(scalar: Scalar) -> Ty {
        Ty {
            scalar,
            lengths: Vec::new(),
        }
    }

    pub fn array(mut self, length: u32) -> Ty {
        self.lengths.push(length);
        self
    }

    pub fn is_scalar(&self) -> bool {
        self.lengths.is_empty()
    }

    pub fn element_count(&self) -> usize {
        self.lengths.iter().fold(1usize, |count, length| {
            count.saturating_mul(*length as usize)
        })
    }
}
