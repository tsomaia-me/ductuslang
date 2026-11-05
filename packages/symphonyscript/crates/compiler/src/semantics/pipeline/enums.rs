use symphonyscript_core::EnumId;

use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::leaf::Leaf;
use super::reason::Reason;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::Op;
use crate::semantics::operators::{EvalTy, Value, ty_name};

impl Core<'_> {
    pub fn ty_text(&self, ty: EvalTy) -> Result<String, Internal> {
        Ok(match ty {
            EvalTy::Enum(id) => self.enum_name(id)?,
            other => ty_name(other).to_string(),
        })
    }

    fn enum_name(&self, id: EnumId) -> Result<String, Internal> {
        Ok(self
            .vocabulary
            .enum_decl(id)
            .ok_or(Internal::missed("enum"))?
            .name
            .clone())
    }

    pub fn is_bare(&self, node: u32) -> bool {
        self.bare.contains(&node)
    }

    pub fn variant_fact(&mut self, node: u32) -> Stage<Fact> {
        let (_, enumeration, variant, _) = self.ir.node(node)?;
        let enum_text = self.name_text(enumeration)?;
        let variant_text = self.name_text(variant)?;
        let Some(id) = self.vocabulary.enum_named(&enum_text) else {
            return Ok(Err(Declined(Reason::EnumFlow)));
        };
        if self.variants.position(id, &variant_text)?.is_none() {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        Ok(Ok(Fact::scalar(Leaf::Enum(Some(id)))))
    }

    pub fn variant_value(&self, node: u32) -> Result<super::core_value::CoreValue, Internal> {
        let (op, a, b, _) = self.ir.node(node)?;
        let (id, text) = if op == Op::Variant {
            let Some(id) = self.vocabulary.enum_named(&self.name_text(a)?) else {
                return Ok(Value::Error);
            };
            (id, self.name_text(b)?)
        } else {
            let Some(id) = self.bare_enums.get(&node).copied() else {
                return Ok(Value::Error);
            };
            (id, self.name_text(a)?)
        };
        Ok(match self.variants.position(id, &text)? {
            Some(position) => Value::Enum(id, position),
            None => Value::Error,
        })
    }

    fn bare_leaves(&self, root: u32) -> Result<Vec<u32>, Internal> {
        let mut found = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let (op, _, b, c) = self.ir.node(node)?;
            if op == Op::If {
                stack.extend([c, b]);
            } else if op == Op::Name && self.is_bare(node) {
                found.push(node);
            }
        }
        Ok(found)
    }

    pub fn resolve_bare(&mut self, root: u32, id: EnumId, draw: bool) -> Result<(), Internal> {
        for node in self.bare_leaves(root)? {
            if self.bare_enums.contains_key(&node) {
                continue;
            }
            let text = self.name_text(self.ir.node(node)?.1)?;
            if self.variants.position(id, &text)?.is_none() {
                if !draw {
                    continue;
                }
                let message = format!("`{text}` is not a variant of `{}`", self.enum_name(id)?);
                self.draw_at(Code::UnknownName, node, message)?;
                continue;
            }
            self.bare_enums.insert(node, id);
        }
        Ok(())
    }

    pub fn enum_gate(&mut self, node: u32, fact: &Fact) -> Stage<()> {
        if !self.enums_seen {
            return Ok(Ok(()));
        }
        let (op, a, b, c) = self.ir.node(node)?;
        for kid in self.operands(node)? {
            let leaf = self.fact(kid)?.leaf;
            let Leaf::Enum(known) = leaf else {
                continue;
            };
            let allowed = match op {
                Op::If => kid != a && (kid == b || kid == c),
                Op::Is | Op::IsNot => true,
                Op::Convert
                | Op::Not
                | Op::Neg
                | Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Div
                | Op::IntDiv
                | Op::Rem
                | Op::Pow
                | Op::Lt
                | Op::Le
                | Op::Gt
                | Op::Ge
                | Op::And
                | Op::Or => known.is_some(),
                Op::Int
                | Op::Float
                | Op::Bool
                | Op::Name
                | Op::Array
                | Op::Index
                | Op::IndexFromEnd
                | Op::Slice
                | Op::For
                | Op::Call
                | Op::NamedCall
                | Op::TyScalar
                | Op::TyArray
                | Op::TyName
                | Op::TyVar
                | Op::To
                | Op::Field
                | Op::Past
                | Op::Stepped
                | Op::Variant
                | Op::Bad
                | Op::Str
                | Op::Match
                | Op::MethodCall
                | Op::NamedMethodCall => false,
            };
            if !allowed || self.in_body(node)?.is_some() {
                return Ok(Err(Declined(Reason::EnumFlow)));
            }
        }
        if matches!(op, Op::Is | Op::IsNot) {
            let (left, right) = (self.fact(a)?.leaf, self.fact(b)?.leaf);
            match (left, right) {
                (Leaf::Enum(Some(id)), Leaf::Enum(None)) => self.resolve_bare(b, id, true)?,
                (Leaf::Enum(None), Leaf::Enum(Some(id))) => self.resolve_bare(a, id, true)?,
                (Leaf::Enum(None), _) | (_, Leaf::Enum(None)) => {
                    return Ok(Err(Declined(Reason::BareVariant)));
                }
                _ => {}
            }
        }
        if op == Op::If
            && let Leaf::Enum(Some(id)) = fact.leaf
        {
            self.resolve_bare(node, id, false)?;
        }
        if op == Op::Name
            && let DeclRef::Let(at) = self.bound(node)?
            && self.has_fact(self.let_root(at)?)?
            && self.fact(self.let_root(at)?)?.leaf == Leaf::Enum(None)
        {
            return Ok(Err(Declined(Reason::BareVariant)));
        }
        if matches!(fact.leaf, Leaf::Enum(_)) && self.in_body(node)?.is_some() {
            return Ok(Err(Declined(Reason::EnumFlow)));
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    pub fn check_variants(&mut self) -> Stage<()> {
        self.enums_seen = !self.bare.is_empty();
        for local in 0..self.ir.op.len() {
            let op = self.ir.op.get(local).copied();
            if matches!(op, Some(Op::Int | Op::Float))
                && let Some(suffix) = self.ir.b.get(local)
                && matches!(
                    self.ir.suffixes.get(*suffix as usize),
                    Some(crate::semantics::ir::Suffix::Unit { .. })
                )
            {
                self.units_seen = true;
            }
            if op != Some(Op::Variant) {
                continue;
            }
            self.enums_seen = true;
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            let (_, enumeration, variant, _) = self.ir.node(node)?;
            let enum_text = self.name_text(enumeration)?;
            let variant_text = self.name_text(variant)?;
            let Some(id) = self.vocabulary.enum_named(&enum_text) else {
                return Ok(Err(Declined(Reason::EnumFlow)));
            };
            if self.variants.position(id, &variant_text)?.is_some() {
                continue;
            }
            let entry = *self.ir.name(variant).ok_or(Internal::missed("IR name"))?;
            let span = self.span(self.file_of(node), entry.lo, entry.hi);
            self.draw(
                Code::UnknownVariant,
                span,
                format!("the enum `{enum_text}` has no variant `{variant_text}`"),
            );
            self.unknown_variants.insert(node);
        }
        if self.enums_seen {
            return Ok(Err(Declined(Reason::EnumFlow)));
        }
        Ok(Ok(()))
    }
}
