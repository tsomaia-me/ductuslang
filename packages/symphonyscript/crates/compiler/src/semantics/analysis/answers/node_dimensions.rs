use super::dimensions::Dimensions;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::resolve::native_layout;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn dimensions(&self, node: NodeId) -> Dimensions {
        if !self.arrays.contains(&node) {
            return Dimensions::Known(0);
        }
        self.memo
            .dimensions
            .get(&node)
            .map_or(Dimensions::Unknown, |known| Dimensions::Known(*known))
    }

    pub(in crate::semantics::analysis) fn dimensions_of(
        &self,
        op: &Op,
    ) -> Result<Dimensions, Internal> {
        Ok(match op {
            Op::Recursive { .. } => Dimensions::Unknown,
            Op::For(for_node) => {
                layout_dimensions(self.slot(for_node.accumulator)?.layout.as_ref())
            }
            Op::Slot(slot) => layout_dimensions(self.slot(*slot)?.layout.as_ref()),
            Op::Element { base, .. } => match self.dimensions(*base) {
                Dimensions::Known(known) => Dimensions::Known(known.saturating_sub(1)),
                Dimensions::Unknown => Dimensions::Unknown,
            },
            Op::Array(elements) => match elements.first() {
                None => Dimensions::Unknown,
                Some(first) => match self.dimensions(*first) {
                    Dimensions::Known(known) => Dimensions::Known(known + 1),
                    Dimensions::Unknown => Dimensions::Unknown,
                },
            },
            Op::Join(lhs, rhs) => match (self.dimensions(*lhs), self.dimensions(*rhs)) {
                (Dimensions::Known(known), _) | (_, Dimensions::Known(known)) if known > 0 => {
                    Dimensions::Known(known)
                }
                (Dimensions::Known(_) | Dimensions::Unknown, _) => Dimensions::Unknown,
            },
            Op::Slice { base, .. } => match self.dimensions(*base) {
                Dimensions::Known(known) if known > 0 => Dimensions::Known(known),
                Dimensions::Known(_) | Dimensions::Unknown => Dimensions::Unknown,
            },
            Op::Builtin(symphonyscript_core::Builtin::Bytes, _) => Dimensions::Known(1),
            Op::History { fallback, cell, .. } => match fallback {
                Some(fallback) => self.dimensions(*fallback),
                None if self.history_holds_array(*cell)? => Dimensions::Unknown,
                None => Dimensions::Known(0),
            },
            Op::Native { native, .. } => {
                let decl = self
                    .vocabulary
                    .native(*native)
                    .ok_or(Internal::missed("native function"))?;
                layout_dimensions(native_layout(&decl.signature.result).as_ref())
            }
            Op::Use(operand) | Op::Chosen(operand) | Op::Annotate(_, operand) => {
                self.dimensions(*operand)
            }
            Op::Read(binding) => match self.memo.binding_layouts.get(binding) {
                Some(layout) => layout_dimensions(Some(layout)),
                None => self.dimensions(self.binding(*binding)?.init),
            },
            Op::If(_, then, otherwise) => {
                Dimensions::agreed([self.dimensions(*then), self.dimensions(*otherwise)])
            }
            Op::Match {
                arms, otherwise, ..
            } => Dimensions::agreed(
                arms.iter()
                    .map(|(_, value)| *value)
                    .chain(*otherwise)
                    .map(|value| self.dimensions(value)),
            ),
            Op::ToRead { attrs, .. } => layout_dimensions(self.to_read_layout(attrs)?.as_ref()),
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Stepped(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Arg(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Pick { .. }
            | Op::Convert(..)
            | Op::Builtin(..) => Dimensions::Known(0),
        })
    }
}

fn layout_dimensions(layout: Option<&Layout>) -> Dimensions {
    Dimensions::Known(layout.map_or(0, |layout| {
        layout.lengths.len() + usize::from(layout.variable)
    }))
}
