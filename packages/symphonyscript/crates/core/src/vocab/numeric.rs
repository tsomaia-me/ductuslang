use super::array_element::ArrayElement;
use super::sealed::Sealed;
use crate::types::Bounded;

pub trait Numeric: Sealed {}

impl Numeric for u8 {}
impl Numeric for i32 {}
impl Numeric for i64 {}
impl Numeric for f32 {}
impl Numeric for f64 {}
impl<T: Numeric + ArrayElement, const N: usize> Numeric for [T; N] {}
impl<T: Numeric + ArrayElement, const N: usize> Numeric for Bounded<T, N> {}
