use std::ops::Range;

use symphonyscript_core::Vocabulary;

use super::faults::Faults;
use super::index::index;
use super::scalar_type::scalar_type;
use crate::semantics::Internal;
use crate::semantics::ir::{
    Expr, ExprId, ExprKind, FnDecl, FnId, Hir, Ident, Input, InputId, LetDecl, LetId, Loc, Param,
    PatternDecl, PatternId, Top, TypeBase, TypeRef,
};
use crate::syntax::ast::{self, AstNode, Item, Name};

pub(crate) struct HirBuilder<'h> {
    pub(super) hir: &'h mut Hir,
    pub(super) file: u32,
    pub(super) faults: Faults,
    pub(super) vocabulary: &'h Vocabulary,
}

impl HirBuilder<'_> {
    pub(super) fn item(&mut self, item: Item<'_>) -> Result<(), Internal> {
        match item {
            Item::Input(node) => {
                let loc = self.loc(node.syntax().range());
                let input = Input {
                    name: self.ident(node.name()),
                    ty: node.ty().map(|ty| self.type_ref(&ty)),
                    value: self.expr(node.value(), loc),
                    loc,
                };
                let id = InputId(index(self.hir.inputs.len()));
                self.hir.inputs.push(input);
                self.hir.top.push(Top::Input(id));
            }
            Item::Let(node) => {
                let loc = self.loc(node.syntax().range());
                let value = match node.match_block() {
                    Some(block) => self.match_block(&block),
                    None => self.expr(node.value(), loc),
                };
                let decl = LetDecl {
                    name: self.ident(node.name()),
                    ty: node.ty().map(|ty| self.type_ref(&ty)),
                    value,
                    loc,
                };
                let id = LetId(index(self.hir.lets.len()));
                self.hir.lets.push(decl);
                self.hir.top.push(Top::Let(id));
            }
            Item::Fn(node) => {
                let loc = self.loc(node.syntax().range());
                let params = node
                    .params()
                    .map(|param| Param {
                        name: self.ident(param.name()),
                        default: param
                            .default_value()
                            .map(|value| self.expr(Some(value), loc)),
                        is_default: false,
                        loc: self.loc(param.syntax().range()),
                    })
                    .collect();
                let (lets, value) = match node.body() {
                    Some(body) => {
                        let lets = body.lets().map(|local| self.local_let(&local)).collect();
                        let value = match body.match_block() {
                            Some(block) => self.match_block(&block),
                            None => self.expr(body.value(), loc),
                        };
                        (lets, value)
                    }
                    None => (Vec::new(), self.push(ExprKind::Error, loc)),
                };
                let decl = FnDecl {
                    name: self.ident(node.name()),
                    params,
                    lets,
                    value,
                    loc,
                };
                let id = FnId(index(self.hir.fns.len()));
                self.hir.fns.push(decl);
                self.hir.top.push(Top::Fn(id));
            }
            Item::Pattern(node) => {
                let loc = self.loc(node.syntax().range());
                let params = node
                    .params()
                    .map(|param| Param {
                        name: self.ident(param.name()),
                        default: param
                            .default_value()
                            .map(|value| self.expr(Some(value), loc)),
                        is_default: param.is_default(),
                        loc: self.loc(param.syntax().range()),
                    })
                    .collect();
                let (lets, body) = match node.body() {
                    Some(block) => {
                        let lets = block.lets().map(|local| self.local_let(&local)).collect();
                        (lets, self.block(block)?)
                    }
                    None => (Vec::new(), self.empty_block()),
                };
                let decl = PatternDecl {
                    name: self.ident(node.name()),
                    params,
                    lets,
                    body,
                    loc,
                };
                let id = PatternId(index(self.hir.patterns.len()));
                self.hir.patterns.push(decl);
                self.hir.top.push(Top::Pattern(id));
            }
            Item::Placement(node) => {
                let id = self.top_placement(&node)?;
                self.hir.top.push(Top::Placement(id));
            }
            Item::Wire(_) => {}
        }
        Ok(())
    }

    pub(super) fn local_let(&mut self, node: &ast::LocalLet<'_>) -> LetDecl {
        let loc = self.loc(node.syntax().range());
        let value = match node.match_block() {
            Some(block) => self.match_block(&block),
            None => self.expr(node.value(), loc),
        };
        LetDecl {
            name: self.ident(node.name()),
            ty: None,
            value,
            loc,
        }
    }

    pub(super) fn type_ref(&mut self, node: &ast::TypeRef<'_>) -> TypeRef {
        let loc = self.loc(node.syntax().range());
        let base = match node.base() {
            Some(token) if self.faults.touch(token.range()) => TypeBase::Error,
            Some(token) => match scalar_type(&token) {
                Some(scalar) => TypeBase::Scalar(scalar),
                None => match self.ident(node.enum_name()) {
                    Some(name) => TypeBase::Enum(name),
                    None => TypeBase::Error,
                },
            },
            None => TypeBase::Error,
        };
        let lengths = node
            .lengths()
            .map(|token| {
                let digits: String = token.text().chars().filter(|c| *c != '_').collect();
                (digits.parse::<u32>().ok(), self.loc(token.range()))
            })
            .collect();
        TypeRef { base, lengths, loc }
    }

    pub(super) fn ident(&self, name: Option<Name<'_>>) -> Option<Ident> {
        let name = name?;
        if self.faults.touch(name.range()) {
            return None;
        }
        Some(Ident {
            text: name.text(),
            loc: self.loc(name.range()),
        })
    }

    pub(super) fn loc(&self, range: Range<u32>) -> Loc {
        Loc::new(self.file, range)
    }

    pub(super) fn push(&mut self, kind: ExprKind, loc: Loc) -> ExprId {
        let id = ExprId(index(self.hir.exprs.len()));
        self.hir.exprs.push(Expr { kind, loc });
        id
    }
}
