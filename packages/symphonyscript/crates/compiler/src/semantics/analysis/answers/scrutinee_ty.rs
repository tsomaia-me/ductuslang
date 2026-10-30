use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::evaluation::own_type;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn scrutinee_ty(
        &self,
        scrutinee: NodeId,
        sig: &Sig,
    ) -> Option<EvalTy> {
        if sig.kind != Kind::Bare {
            return Some(own_type(sig, EvalTy::Exact));
        }
        self.memo
            .bare_scrutinees
            .get(&scrutinee)
            .map(|id| EvalTy::Enum(*id))
    }

    pub(in crate::semantics::analysis) fn choice_ty(
        &self,
        scrutinee: NodeId,
        sig: &Sig,
        labels: &[NodeId],
    ) -> Result<Option<EvalTy>, Internal> {
        if let Some(ty) = self.scrutinee_ty(scrutinee, sig) {
            return Ok(Some(ty));
        }
        let mut names = Vec::new();
        for label in labels {
            let Op::Bare(name) = &self.node(*label)?.op else {
                return Ok(None);
            };
            names.push(name.as_str());
        }
        let mut seen = FastSet::default();
        let mut stack = vec![scrutinee];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::ScrutineeTyChoiceTy.step();
            if !seen.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::Bare(name) => names.push(name.as_str()),
                Op::Use(inner) | Op::Chosen(inner) => stack.push(*inner),
                Op::If(_, then, otherwise) => stack.extend([*then, *otherwise]),
                Op::Match {
                    arms, otherwise, ..
                } => stack.extend(arms.iter().map(|(_, value)| *value).chain(*otherwise)),
                Op::Pick { items, .. } | Op::Array(items) => stack.extend(items),
                Op::Element { base, .. } | Op::Slice { base, .. } => stack.push(*base),
                Op::Join(lhs, rhs) => stack.extend([*lhs, *rhs]),
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
                | Op::Convert(..)
                | Op::Annotate(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::ToRead { .. }
                | Op::History { .. }
                | Op::Native { .. } => return Ok(None),
            }
        }
        for index in 0..self.vocabulary.enums().len() {
            let id = self
                .vocabulary
                .enum_at(index)
                .ok_or(Internal::missed("enum"))?;
            let mut holds = true;
            for name in &names {
                holds &= self.variants.position(id, name)?.is_some();
            }
            if holds {
                return Ok(Some(EvalTy::Enum(id)));
            }
        }
        Ok(None)
    }
}
