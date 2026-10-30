use super::loop_items::LoopItems;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::ids::SlotId;

pub(crate) struct LoopRun {
    pub items: LoopItems,
    pub next: usize,
    pub index: Option<SlotId>,
    pub element: Option<SlotId>,
    pub accumulator: SlotId,
    pub values: [CVal; 3],
}

impl LoopRun {
    pub fn value(&self, slot: SlotId) -> Option<&CVal> {
        let [index, element, accumulator] = &self.values;
        if self.index == Some(slot) {
            Some(index)
        } else if self.element == Some(slot) {
            Some(element)
        } else if self.accumulator == slot {
            Some(accumulator)
        } else {
            None
        }
    }
}
