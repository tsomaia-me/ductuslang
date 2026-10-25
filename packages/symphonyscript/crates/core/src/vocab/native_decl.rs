use alloc::string::String;

use super::native_fn::NativeFn;
use super::native_signature::NativeSignature;

#[derive(Clone, Debug)]
pub struct NativeDecl {
    pub name: String,
    pub signature: NativeSignature,
    pub function: NativeFn,
}
