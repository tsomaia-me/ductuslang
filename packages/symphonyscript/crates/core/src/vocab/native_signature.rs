use alloc::vec::Vec;

use super::native_type::NativeType;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NativeSignature {
    pub params: Vec<NativeType>,
    pub result: NativeType,
}

impl NativeSignature {
    pub fn new(params: impl IntoIterator<Item = NativeType>, result: NativeType) -> Self {
        Self {
            params: params.into_iter().collect(),
            result,
        }
    }
}
