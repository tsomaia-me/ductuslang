use alloc::boxed::Box;
use alloc::vec::Vec;

use super::sealed::Sealed;
use crate::types::{Ty, Value, canonical_f32, canonical_f64};

pub trait ArrayElement: Sealed + Sized {
    fn element_ty() -> Ty;
    fn push_elements(self, out: &mut Vec<Value>);
}

impl Sealed for i32 {}
impl Sealed for i64 {}
impl Sealed for f32 {}
impl Sealed for f64 {}
impl Sealed for bool {}
impl Sealed for &'static str {}
impl<T: ArrayElement, const N: usize> Sealed for [T; N] {}

impl ArrayElement for i32 {
    fn element_ty() -> Ty {
        Ty::I32
    }

    fn push_elements(self, out: &mut Vec<Value>) {
        out.push(Value::I32(self));
    }
}

impl ArrayElement for i64 {
    fn element_ty() -> Ty {
        Ty::I64
    }

    fn push_elements(self, out: &mut Vec<Value>) {
        out.push(Value::I64(self));
    }
}

impl ArrayElement for f32 {
    fn element_ty() -> Ty {
        Ty::F32
    }

    fn push_elements(self, out: &mut Vec<Value>) {
        out.push(Value::F32(canonical_f32(self)));
    }
}

impl ArrayElement for f64 {
    fn element_ty() -> Ty {
        Ty::F64
    }

    fn push_elements(self, out: &mut Vec<Value>) {
        out.push(Value::F64(canonical_f64(self)));
    }
}

impl ArrayElement for bool {
    fn element_ty() -> Ty {
        Ty::Bool
    }

    fn push_elements(self, out: &mut Vec<Value>) {
        out.push(Value::Bool(self));
    }
}

impl<T: ArrayElement, const N: usize> ArrayElement for [T; N] {
    fn element_ty() -> Ty {
        Ty::Array(Box::new(T::element_ty()), N as u32)
    }

    fn push_elements(self, out: &mut Vec<Value>) {
        for element in self {
            element.push_elements(out);
        }
    }
}
