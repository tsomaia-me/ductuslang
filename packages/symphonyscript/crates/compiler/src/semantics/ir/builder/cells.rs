use std::ops::Range;

use super::hir_builder::{HirBuilder, digits_value};
use super::index::index;
use crate::diagnostics::Code;
use crate::semantics::ir::{CellWord, ExprKind, Input, InputId, StateDecl, StateId, Top, TypeBase};
use crate::syntax::SyntaxKind;
use crate::syntax::ast::{self, AstNode};

impl HirBuilder<'_> {
    pub(super) fn cell(&mut self, node: &ast::CellDecl<'_>) {
        let loc = self.loc(node.syntax().range());
        let rule = node.has_rule();
        let word = match node.word().map(|token| token.kind()) {
            Some(SyntaxKind::StreamKw) => CellWord::Stream,
            Some(SyntaxKind::ChannelKw) => CellWord::Channel,
            Some(_) | None => CellWord::State,
        };
        if let Some(clock) = node.clock()
            && rule
        {
            self.draw(
                Code::UnbuiltConstruct,
                clock.syntax().range(),
                "a clock (SPEC §6.6) is a construct this compiler does not build",
            );
        }
        let capacity = node.capacity().and_then(|token| {
            (!self.faults.touch(token.range()))
                .then(|| digits_value(token.text()))
                .flatten()
        });
        let misplaced_capacity = node.has_capacity() && (rule || word == CellWord::State);
        if misplaced_capacity && let Some(range) = brackets(node) {
            let summary = if rule {
                "`[N]` states the capacity of a host-fed `stream` or `channel`, and this cell is derived"
            } else {
                "`[N]` states the capacity of a host-fed `stream` or `channel`, and this cell is a `state`"
            };
            self.draw(Code::CellCapacity, range, summary);
        }
        let name = self.ident(node.name());
        let ty = node.ty().map(|ty| {
            let mut built = self.type_ref(&ty);
            let after = ty.syntax().range().end;
            if self
                .faults
                .first_in(after..annotation_end(node).max(after))
                .is_some()
            {
                built.base = TypeBase::Error;
            }
            built
        });
        if !rule && let Some(init) = node.init() {
            let value = self.expr(init.value(), loc);
            let input = Input {
                name,
                word,
                capacity: capacity
                    .filter(|_| word != CellWord::State)
                    .map(|value| u32::try_from(value).unwrap_or(u32::MAX)),
                ty,
                value,
                loc,
            };
            let id = InputId(index(self.hir.inputs.len()));
            self.hir.inputs.push(input);
            self.hir.top.push(Top::Input(id));
            return;
        }
        let init = node
            .init()
            .map(|init| self.expr(init.value(), self.loc(init.syntax().range())));
        let clock = node
            .clock()
            .map(|clock| {
                clock
                    .entries()
                    .map(|entry| self.expr(Some(entry), loc))
                    .collect()
            })
            .unwrap_or_default();
        let value = match node.match_block() {
            Some(block) => self.match_block(&block),
            None if rule => self.expr(node.value(), loc),
            None => self.push(ExprKind::Error, loc),
        };
        let decl = StateDecl {
            name,
            word,
            init,
            clock,
            ty,
            value,
            loc,
        };
        let id = StateId(index(self.hir.states.len()));
        self.hir.states.push(decl);
        self.hir.top.push(Top::State(id));
    }

    pub(super) fn draw(&mut self, code: Code, range: Range<u32>, summary: &str) {
        if !self.faults.touch(range.clone()) {
            let loc = self.loc(range);
            self.hir.drawn.push((code, loc, summary.to_string()));
        }
    }
}

fn brackets(node: &ast::CellDecl<'_>) -> Option<Range<u32>> {
    let mut tokens = node
        .syntax()
        .children_with_tokens()
        .filter_map(|element| element.into_token());
    let open = tokens.find(|token| token.kind() == SyntaxKind::LBracket)?;
    let close = tokens.find(|token| token.kind() == SyntaxKind::RBracket)?;
    Some(open.range().start..close.range().end)
}

fn annotation_end(node: &ast::CellDecl<'_>) -> u32 {
    let eq = node
        .syntax()
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| token.kind() == SyntaxKind::Eq)
        .map(|token| token.range().start);
    let clock = node.clock().map(|clock| clock.syntax().range().start);
    clock.or(eq).unwrap_or(node.syntax().range().end)
}
