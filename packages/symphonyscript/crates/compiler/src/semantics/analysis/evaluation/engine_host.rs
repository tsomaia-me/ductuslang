use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_array::CArray;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::lengths_of::lengths_of;
use crate::semantics::operators::OperatorHost;

impl OperatorHost for Engine<'_> {
    type Node = NodeId;
    type Array = CArray;
    type Reference = ItemRef;

    fn fault(&mut self, code: Code, node: NodeId, message: String) -> Result<(), Internal> {
        self.error_at_node(code, node, message)
    }

    fn fault_helped(
        &mut self,
        code: Code,
        node: NodeId,
        message: String,
        help: String,
    ) -> Result<(), Internal> {
        self.error_at_node_helped(code, node, Vec::new(), message, help)
    }

    fn within_value_limit(
        &mut self,
        node: NodeId,
        value: CVal,
        built: &str,
    ) -> Result<CVal, Internal> {
        Engine::within_value_limit(self, node, value, built)
    }

    fn join(&mut self, left: &CArray, right: &CArray) -> Result<CArray, Code> {
        CArray::join(left, right)
    }

    fn slice(&mut self, base: &CArray, start: usize, end: usize) -> CArray {
        CArray::slice(base, start, end)
    }

    fn length(&self, array: &CArray) -> usize {
        array.len()
    }

    fn element(&mut self, array: &CArray, position: usize) -> Option<CVal> {
        array.get(position).cloned()
    }

    fn array(&mut self, items: Vec<CVal>) -> CArray {
        CArray::new(items)
    }

    fn lengths(&mut self, value: &CVal) -> Vec<u32> {
        lengths_of(value)
    }
}
