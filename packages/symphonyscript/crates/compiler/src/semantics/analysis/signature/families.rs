use symphonyscript_core::FamilyId;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::sig::Sig;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn one_family(
        &mut self,
        node: NodeId,
        operands: &[(NodeId, Sig)],
    ) -> Result<Option<Option<FamilyId>>, Internal> {
        let mut family: Option<FamilyId> = None;
        for (_, sig) in operands.iter().filter(|(_, sig)| !sig.open) {
            match (family, sig.family) {
                (Some(existing), Some(other)) if existing != other => {
                    let names = [existing, other]
                        .iter()
                        .map(|id| {
                            self.vocabulary
                                .family(*id)
                                .map_or(String::new(), |decl| format!("`{}`", decl.name))
                        })
                        .collect::<Vec<_>>()
                        .join(" and ");
                    self.error_at_node(
                        Code::MixedFamilies,
                        node,
                        format!(
                            "these values belong to the unit families {names}, which never convert"
                        ),
                    )?;
                    return Ok(None);
                }
                (None, Some(other)) => family = Some(other),
                _ => {}
            }
        }
        if let Some(family) = family {
            for (operand, sig) in operands {
                if sig.family.is_none() && sig.is_numeric() && !sig.open {
                    self.memo.takes_family.insert(*operand, family);
                    if let Some(template) = self.template_of(self.node(*operand)?.scope)? {
                        self.memo
                            .template_takers
                            .entry(template)
                            .or_default()
                            .push(*operand);
                    }
                }
            }
        }
        Ok(Some(family))
    }
}
