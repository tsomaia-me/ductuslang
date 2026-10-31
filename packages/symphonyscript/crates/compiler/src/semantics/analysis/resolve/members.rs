use super::found::Found;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::val::Val;
use crate::semantics::builtin::Builtin;
use crate::semantics::ir::{ExprId, ExprKind, Ident, Loc};

pub(super) enum Member {
    Declined,
    Pending,
    Read(Val),
}

pub(super) const QUALIFIED_NAME: &str = "a qualified name (SPEC §15.3)";

pub(super) const ARRAY_MEMBERS: [&str; 4] = ["size", "capacity", "bytes", "map"];

impl Engine<'_> {
    pub(super) fn member(
        &mut self,
        base: ExprId,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Member, Internal> {
        let text = name.text.as_str();
        if text == "dropped"
            && let ExprKind::Name(cell) = &self.hir.expr(base)?.kind
            && matches!(
                self.find(cell, scope)?,
                Found::Query(Query::Input(_) | Query::State(_) | Query::Let(_, LetRef::Top(_)))
            )
        {
            self.unbuilt_member(loc, scope, "a cell's `dropped` (SPEC §12.1)")?;
            return Ok(Member::Read(Val::Error));
        }
        if !ARRAY_MEMBERS.contains(&text) || matches!(self.hir.expr(base)?.kind, ExprKind::To) {
            return Ok(Member::Declined);
        }
        let Some(val) = self.val(Query::Expr(base, scope), need) else {
            return Ok(Member::Pending);
        };
        let node = match &val {
            Val::Array(_) => None,
            Val::Scalar(node) => Some(*node),
            Val::Error => return Ok(Member::Declined),
        };
        if let Some(node) = node
            && !self.arrays.contains(&node)
        {
            let Some(sig) = self.sig(node, need) else {
                return Ok(Member::Pending);
            };
            if sig.kind != Kind::Str || text == "map" {
                return Ok(Member::Declined);
            }
            if text == "bytes" {
                return Ok(Member::Read(self.bytes_val(val, loc, scope)?));
            }
            self.unbuilt_member(loc, scope, &format!("a string's `{text}` (SPEC §5.7)"))?;
            return Ok(Member::Read(Val::Error));
        }
        Ok(match text {
            "bytes" => Member::Read(self.bytes_val(val, loc, scope)?),
            "map" => {
                self.unbuilt_member(loc, scope, "an array's `map` (SPEC §5.7)")?;
                Member::Read(Val::Error)
            }
            _ => match self.length_val(val, loc, scope, need)? {
                None => Member::Pending,
                Some(Val::Scalar(measured)) if text == "capacity" => {
                    let op = self.node(measured)?.op.clone();
                    if let Op::Builtin(Builtin::Len, operands) = op {
                        Member::Read(self.scalar(
                            Op::Builtin(Builtin::Capacity, operands),
                            loc,
                            scope,
                        )?)
                    } else {
                        Member::Read(Val::Scalar(measured))
                    }
                }
                Some(length) => Member::Read(length),
            },
        })
    }

    pub(super) fn unbuilt_member(
        &mut self,
        loc: Loc,
        scope: ScopeId,
        construct: &str,
    ) -> Result<(), Internal> {
        self.error(
            Code::UnbuiltConstruct,
            loc,
            scope,
            format!("{construct} is a construct this compiler does not build"),
        )
    }
}

impl Engine<'_> {
    pub(super) fn dot_member(
        &mut self,
        receiver: ExprId,
        name: &str,
        (loc, call): (Loc, Loc),
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Member, Internal> {
        if matches!(name, "stepped" | "dropped")
            && let ExprKind::Name(cell) = &self.hir.expr(receiver)?.kind
            && matches!(
                self.find(cell, scope)?,
                Found::Query(Query::Input(_) | Query::State(_) | Query::Let(_, LetRef::Top(_)))
            )
        {
            return self.uncallable(loc, scope);
        }
        let Some(val) = self.val(Query::Expr(receiver, scope), need) else {
            return Ok(Member::Pending);
        };
        let node = match &val {
            Val::Array(_) => None,
            Val::Scalar(node) => Some(*node),
            Val::Error => return Ok(Member::Read(Val::Error)),
        };
        let array = node.is_none_or(|node| self.arrays.contains(&node));
        if array && name == "map" {
            self.unbuilt_member(call, scope, "an array's `map` (SPEC §5.7)")?;
            return Ok(Member::Read(Val::Error));
        }
        let kind = match node {
            Some(node) if !array => match self.sig(node, need) {
                Some(sig) => Some(sig.kind),
                None => return Ok(Member::Pending),
            },
            Some(_) | None => None,
        };
        let member = matches!(name, "size" | "capacity" | "bytes");
        if (member && (array || kind == Some(Kind::Str))) || kind == Some(Kind::Node) {
            return self.uncallable(loc, scope);
        }
        Ok(Member::Declined)
    }

    pub(super) fn uncallable(&mut self, loc: Loc, scope: ScopeId) -> Result<Member, Internal> {
        self.error(
            Code::NotCallable,
            loc,
            scope,
            "only a function or a built-in is called",
        )?;
        Ok(Member::Read(Val::Error))
    }
}
