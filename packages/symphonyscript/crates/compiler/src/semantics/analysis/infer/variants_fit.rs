use symphonyscript_core::EnumId;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;

impl Engine<'_> {
    pub(super) fn variants_fit(&mut self, root: NodeId, expected: EnumId) -> Result<(), Internal> {
        let mut stack = vec![(root, expected)];
        while let Some((node, expected)) = stack.pop() {
            if !self.typing.variants_fit.insert((node, expected)) {
                continue;
            }
            if let Some(group) = self.memo.groups.get(&node).copied() {
                let own = match self.known_sig(group)?.kind {
                    Kind::Enum(id) => id,
                    Kind::Int
                    | Kind::Float
                    | Kind::Bool
                    | Kind::Str
                    | Kind::Node
                    | Kind::Bare
                    | Kind::Any
                    | Kind::Error => expected,
                };
                stack.push((group, own));
            }
            let op = self.node(node)?.op.clone();
            if self.known_sig(node)?.reactive && !matches!(op, Op::Array(_)) {
                continue;
            }
            match op {
                Op::Bare(name) => {
                    if self.variants.position(expected, &name)?.is_none() {
                        let message = format!(
                            "`{name}` is not a variant of `{}`",
                            self.enum_decl(expected)?.name
                        );
                        self.error_at_node(Code::UnknownName, node, message)?;
                    }
                }
                Op::Use(inner) | Op::Chosen(inner) => stack.push((inner, expected)),
                Op::If(_, then, otherwise) => {
                    stack.extend([(then, expected), (otherwise, expected)]);
                }
                Op::Match {
                    arms, otherwise, ..
                } => {
                    stack.extend(arms.iter().map(|(_, value)| (*value, expected)));
                    stack.extend(otherwise.map(|value| (value, expected)));
                }
                Op::Pick { items, .. } | Op::Array(items) => {
                    stack.extend(items.into_iter().map(|item| (item, expected)));
                }
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Pi
                | Op::NodeRef(_)
                | Op::Read(_)
                | Op::Arg(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::Element { .. }
                | Op::Convert(..)
                | Op::Annotate(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::ToRead { .. } => {}
            }
        }
        Ok(())
    }
}
