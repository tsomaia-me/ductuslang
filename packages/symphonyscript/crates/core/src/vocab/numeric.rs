use super::array_element::ArrayElement;
use super::sealed::Sealed;

pub trait Numeric: Sealed {}

impl Numeric for i32 {}
impl Numeric for i64 {}
impl Numeric for f32 {}
impl Numeric for f64 {}
impl<T: Numeric + ArrayElement, const N: usize> Numeric for [T; N] {}
