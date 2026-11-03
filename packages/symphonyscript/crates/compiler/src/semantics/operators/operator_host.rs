use super::value::Value;
use crate::diagnostics::Code;
use crate::semantics::Internal;

pub(crate) type Held<H> = Value<<H as OperatorHost>::Array, <H as OperatorHost>::Reference>;

pub(crate) trait OperatorHost {
    type Node: Copy;
    type Array: Clone;
    type Reference: Clone;

    fn fault(&mut self, code: Code, node: Self::Node, message: String) -> Result<(), Internal>;

    fn fault_helped(
        &mut self,
        code: Code,
        node: Self::Node,
        message: String,
        help: String,
    ) -> Result<(), Internal>;

    fn within_value_limit(
        &mut self,
        node: Self::Node,
        value: Held<Self>,
        built: &str,
    ) -> Result<Held<Self>, Internal>;

    fn join(&mut self, left: &Self::Array, right: &Self::Array) -> Result<Self::Array, Code>;

    fn slice(&mut self, base: &Self::Array, start: usize, end: usize) -> Self::Array;

    fn length(&self, array: &Self::Array) -> usize;

    fn element(&mut self, array: &Self::Array, position: usize) -> Option<Held<Self>>;

    fn array(&mut self, items: Vec<Held<Self>>) -> Self::Array;

    fn lengths(&mut self, value: &Held<Self>) -> Vec<u32>;
}
