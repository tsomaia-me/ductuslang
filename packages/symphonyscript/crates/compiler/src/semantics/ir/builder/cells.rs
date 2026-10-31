use std::ops::Range;

use super::hir_builder::{HirBuilder, digits_value};
use super::index::index;
use crate::semantics::ir::{ExprKind, Input, InputId, StateDecl, StateId, Top};
use crate::syntax::SyntaxKind;
use crate::syntax::ast::{self, AstNode};

impl HirBuilder<'_> {
    pub(super) fn cell(&mut self, node: &ast::CellDecl<'_>) {
        let loc = self.loc(node.syntax().range());
        let rule = node.has_rule();
        let mut unbuilt = false;
        if let Some(word) = node.word()
            && word.kind() != SyntaxKind::StateKw
        {
            unbuilt |= self.unbuilt(word.range(), &format!("a `{}` cell", word.text()), "6.1");
        }
        if let Some(clock) = node.clock() {
            unbuilt |= self.unbuilt(clock.syntax().range(), "a clock", "6.6");
        }
        if let Some(init) = node.init()
            && rule
        {
            unbuilt |= self.unbuilt(init.syntax().range(), "a derived cell's init", "6.6");
        }
        if node.has_capacity() && !rule {
            let range = node.syntax().range();
            unbuilt |= self.unbuilt(range, "a host-fed cell's capacity `[N]`", "6.1");
        }
        if !rule {
            let value = match node.init() {
                Some(init) if !unbuilt => self.expr(init.value(), loc),
                Some(_) | None => self.push(ExprKind::Error, loc),
            };
            let input = Input {
                name: self.ident(node.name()),
                ty: node.ty().map(|ty| self.type_ref(&ty)),
                value,
                loc,
            };
            let id = InputId(index(self.hir.inputs.len()));
            self.hir.inputs.push(input);
            self.hir.top.push(Top::Input(id));
            return;
        }
        let value = match node.match_block() {
            _ if unbuilt => self.push(ExprKind::Error, loc),
            Some(block) => self.match_block(&block),
            None => self.expr(node.value(), loc),
        };
        let depth = node.capacity().map(|token| {
            let value = if self.faults.touch(token.range()) {
                None
            } else {
                digits_value(token.text())
            };
            (value, self.loc(token.range()))
        });
        let decl = StateDecl {
            name: self.ident(node.name()),
            depth,
            ty: node.ty().map(|ty| self.type_ref(&ty)),
            value,
            loc,
        };
        let id = StateId(index(self.hir.states.len()));
        self.hir.states.push(decl);
        self.hir.top.push(Top::State(id));
    }

    fn unbuilt(&mut self, range: Range<u32>, construct: &str, section: &str) -> bool {
        if !self.faults.touch(range.clone()) {
            let loc = self.loc(range);
            self.hir.unbuilt.push((
                loc,
                format!(
                    "{construct} (SPEC §{section}) is a construct this compiler does not build"
                ),
            ));
        }
        true
    }
}
