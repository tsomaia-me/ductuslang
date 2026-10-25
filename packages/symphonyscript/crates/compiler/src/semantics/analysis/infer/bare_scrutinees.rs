use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;

impl Engine<'_> {
    pub(super) fn bare_scrutinees(&mut self) -> Result<(), Internal> {
        let mut matches = self.memo.bare_matches.clone();
        matches.sort();
        matches.dedup();
        if matches.is_empty() {
            return Ok(());
        }
        for node in matches {
            let Op::Match {
                scrutinee,
                arms,
                otherwise,
            } = self.node(node)?.op.clone()
            else {
                return Err(Internal::missed("match"));
            };
            let source = self.through_uses(scrutinee)?;
            let found = match self.node(source)?.op {
                Op::Read(binding) => match self.typing.own.get(&binding) {
                    Some(Scalar::Enum(id)) => Some(*id),
                    Some(_) | None if self.typing.untyped.contains(&binding) => continue,
                    Some(_) | None => None,
                },
                _ => match self.memo.bare_enums.get(&source) {
                    Some(id) => Some(*id),
                    None if self.typing.untyped_values.contains(&source) => continue,
                    None => None,
                },
            };
            let Some(id) = found else {
                self.error_at_node(
                    Code::MatchScrutinee,
                    scrutinee,
                    "a `match` selects by an enum value, an integer or a `bool`",
                )?;
                continue;
            };
            let mut sig = self.known_sig(scrutinee)?;
            sig.kind = Kind::Enum(id);
            if self.match_labels(node, &arms, otherwise, &sig)? {
                self.memo.bare_scrutinees.insert(scrutinee, id);
            }
        }
        Ok(())
    }
}
