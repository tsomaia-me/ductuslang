use std::ops::Range;

use super::hir_builder::HirBuilder;
use super::index::index;
use crate::semantics::ir::Top;
use crate::syntax::ast::{self, AstNode, Name};
use crate::syntax::{NodeView, SyntaxKind};

impl HirBuilder<'_> {
    pub(super) fn placeholder(&mut self, name: Option<Name<'_>>) {
        if let Some(name) = self.ident(name) {
            let id = index(self.hir.placeholders.len());
            self.hir.placeholders.push(name);
            self.hir.top.push(Top::Placeholder(id));
        }
    }

    pub(super) fn unbuilt_fn(&mut self, node: &ast::FnDecl<'_>) -> bool {
        let mut parts: Vec<(Range<u32>, &str)> = Vec::new();
        if let Some(params) = node.generic_params() {
            parts.push((params.syntax().range(), GENERIC));
        }
        for param in node.params() {
            if let Some(marker) = param.marker() {
                parts.push((marker.range(), "a `cell` parameter (SPEC §6.3)"));
            }
            if let Some(ty) = param.ty() {
                parts.push((
                    ty.syntax().range(),
                    "an annotation on a parameter (SPEC §6.3)",
                ));
            }
        }
        if let Some(result) = node.result() {
            parts.push((result.syntax().range(), "a result annotation (SPEC §6.3)"));
        }
        for local in node.body().into_iter().flat_map(|body| body.lets()) {
            if let Some(ty) = local.ty() {
                parts.push((ty.syntax().range(), LOCAL_ANNOTATION));
            }
        }
        self.draw_parts(node.syntax().range(), parts)
    }

    pub(super) fn unbuilt_pattern(&mut self, node: &ast::PatternDecl<'_>) -> bool {
        let mut parts: Vec<(Range<u32>, &str)> = Vec::new();
        if let Some(params) = node.generic_params() {
            parts.push((params.syntax().range(), GENERIC));
        }
        for param in node.params() {
            if param.is_children() {
                parts.push((param.syntax().range(), "a `children` parameter (SPEC §6.4)"));
                continue;
            }
            let markers: Vec<Range<u32>> = param.markers().map(|marker| marker.range()).collect();
            if let (Some(first), Some(last)) = (markers.first(), markers.last()) {
                parts.push((first.start..last.end, "a parameter marker (SPEC §6.4)"));
            }
            if let Some(ty) = param.ty() {
                parts.push((
                    ty.syntax().range(),
                    "an annotation on a pattern parameter (SPEC §6.4)",
                ));
            }
        }
        if let Some(placement_type) = node.placement_type() {
            parts.push((
                placement_type.syntax().range(),
                "a pattern's placement type (SPEC §6.4)",
            ));
        }
        if let Some(body) = node.body() {
            for cell in body.cells() {
                parts.push((
                    trimmed(cell.syntax()),
                    "a cell in a pattern body (SPEC §6.4)",
                ));
            }
            for local in body.lets() {
                if let Some(ty) = local.ty() {
                    parts.push((ty.syntax().range(), LOCAL_ANNOTATION));
                }
            }
        }
        self.draw_parts(node.syntax().range(), parts)
    }

    fn draw_parts(&mut self, head: Range<u32>, parts: Vec<(Range<u32>, &str)>) -> bool {
        let unbuilt = !parts.is_empty();
        if self.faults.first_in(head).is_some() {
            return unbuilt;
        }
        for (range, construct) in parts {
            self.draw_unbuilt(range, construct);
        }
        unbuilt
    }
}

const GENERIC: &str = "a list of bracket parameters (SPEC §6.8)";

const LOCAL_ANNOTATION: &str = "an annotation on a local `let` (SPEC §6.3)";

pub(super) fn trimmed(node: &NodeView<'_>) -> Range<u32> {
    let start = node.range().start;
    let mut token = node.last_token();
    while let Some(current) = token {
        if current.range().start < start {
            break;
        }
        let kind = current.kind();
        if !kind.is_trivia()
            && !matches!(
                kind,
                SyntaxKind::Newline | SyntaxKind::Indent | SyntaxKind::Dedent
            )
        {
            return start..current.range().end;
        }
        token = current.prev_token();
    }
    node.range()
}

impl HirBuilder<'_> {
    pub(super) fn draw_item(&mut self, node: &NodeView<'_>, construct: &str) {
        if self.faults.first_in(node.range()).is_none() {
            self.draw_unbuilt(trimmed(node), construct);
        }
    }
}
