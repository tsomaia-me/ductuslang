mod blocks;
mod checkpoint;
mod expressions;
mod items;
mod placements;
mod task;
mod tree_builder;

use rowan::GreenNode;
use symphonyscript_core::{Span, Vocabulary};

use self::checkpoint::Checkpoint;
use self::task::Task;
use self::tree_builder::TreeBuilder;
use super::lexed::Lexed;
use super::lexer::{Layout, Lexer, Suspension};
use super::nfc::nfc;
use super::syntax_kind::SyntaxKind;
use super::token::Token;
use crate::diagnostics::{Code, Diagnostic};
use crate::source::Source;

pub(crate) struct Parser<'a> {
    source: &'a Source,
    vocabulary: &'a Vocabulary,
    tokens: Vec<Token>,
    depths: Vec<u32>,
    suspended: Vec<Suspension>,
    lexer_diagnostics: Vec<Diagnostic>,
    pos: usize,
    builder: TreeBuilder,
    diagnostics: Vec<Diagnostic>,
    line_error: bool,
    previous_line_error: bool,
    tasks: Vec<Task>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(
        source: &'a Source,
        vocabulary: &'a Vocabulary,
        lexed: Lexed,
        layout: Layout,
    ) -> Self {
        Self {
            source,
            vocabulary,
            tokens: lexed.tokens,
            depths: layout.depths,
            suspended: layout.suspended,
            lexer_diagnostics: lexed.diagnostics,
            pos: 0,
            builder: TreeBuilder::new(),
            diagnostics: Vec::new(),
            line_error: false,
            previous_line_error: false,
            tasks: Vec::new(),
        }
    }

    pub(crate) fn run(mut self) -> (GreenNode, Vec<Diagnostic>) {
        self.builder.start_node(SyntaxKind::File);
        self.tasks.push(Task::File);
        while let Some(task) = self.tasks.pop() {
            self.step(task);
        }
        self.flush_trivia();
        while self.pos < self.tokens.len() {
            self.raw_bump();
        }
        self.builder.finish_node();
        let green = self.builder.finish();
        let mut diagnostics = self.lexer_diagnostics;
        diagnostics.extend(self.diagnostics);
        diagnostics.sort_by_key(|diagnostic| diagnostic.span.range.start);
        (green, diagnostics)
    }

    fn then<const N: usize>(&mut self, tasks: [Task; N]) {
        self.tasks.extend(tasks.into_iter().rev());
    }

    fn step(&mut self, task: Task) {
        match task {
            Task::Finish => self.finish(),
            Task::FinishLine(code) => self.finish_line(code),
            Task::FinishLineFinish(code) => {
                self.finish_line(code);
                self.finish();
            }
            Task::Close { kind, code, what } => {
                self.expect(kind, code, what);
                self.finish();
            }
            Task::File => self.file_step(),
            Task::TopItem => self.top_item(),
            Task::InputDecl => self.input_decl(),
            Task::LetDecl(kind) => self.let_decl(kind),
            Task::FnDecl => self.fn_decl(),
            Task::Param { pattern } => self.param(pattern),
            Task::ParamAfter { pattern } => self.param_after(pattern),
            Task::FnAfterParams => self.fn_after_params(),
            Task::FnBody => self.fn_body(),
            Task::FnLets => self.fn_lets(),
            Task::FnBodyBlockEnd => self.fn_body_block_end(),
            Task::PatternDecl => self.pattern_decl(),
            Task::PatternAfterParams => self.pattern_after_params(),
            Task::Block { pattern } => self.block(pattern),
            Task::PatternLets => self.pattern_lets(),
            Task::BlockItems { pattern, items } => self.block_items(pattern, items),
            Task::BodyItem => self.body_item(),
            Task::ForBlock => self.for_block(),
            Task::ForAfterIterable => self.for_after_iterable(),
            Task::Iterable => self.iterable(),
            Task::IterableEnd(checkpoint) => self.iterable_end(checkpoint),
            Task::WhenBlock => self.when_block(),
            Task::WhenArms { arms } => self.when_arms(arms),
            Task::WhenAfterElse { arms } => self.when_after_else(arms),
            Task::GuardAfter { arms } => self.guard_after(arms),
            Task::WhenAfterCondition => self.when_after_condition(),
            Task::WhenElseOptional => self.when_else_optional(),
            Task::WhenElse => self.when_else(),
            Task::MatchBlock => self.match_block(),
            Task::MatchAfterScrutinee => self.match_after_scrutinee(),
            Task::MatchArms { arms, after_else } => self.match_arms(arms, after_else),
            Task::MatchArmEnd { arms, after_else } => self.match_arm_end(arms, after_else),
            Task::TopPlacement => self.top_placement(),
            Task::WirePlacement => self.wire_placement(),
            Task::WireAfterDefault => self.wire_after_default(),
            Task::WireAfterGate => self.wire_after_gate(),
            Task::WireAfterAttrs => self.wire_after_attrs(),
            Task::PlacementTail => self.placement_tail(),
            Task::PlacementTailAttrs => self.placement_tail_attrs(),
            Task::PlacementTailBody => self.placement_tail_body(),
            Task::Line => self.line(),
            Task::LineAfterHead(checkpoint) => self.line_after_head(checkpoint),
            Task::LineMembers => self.line_members(),
            Task::Members(kind) => self.members(kind),
            Task::MembersLoop => self.members_loop(),
            Task::Member => self.member(),
            Task::ParenAfterHead => self.paren_after_head(),
            Task::ParenAfterGate => self.paren_after_gate(),
            Task::ParenFinishPlacement => self.paren_finish_placement(),
            Task::PlacementHead => self.placement_head(),
            Task::DefaultArg => self.default_arg(),
            Task::DefaultAfterAtom => self.default_after_atom(),
            Task::Atom => self.atom(),
            Task::Gate => self.gate(),
            Task::Attrs => self.attrs(),
            Task::AttrsLoop => self.attrs_loop(),
            Task::Bundle => self.bundle(),
            Task::BundleLoop { members } => self.bundle_loop(members),
            Task::BundleForAfterIterable { members } => self.bundle_for_after_iterable(members),
            Task::BundleForMembers { members } => self.bundle_for_members(members),
            Task::BundleEnd { members } => self.bundle_end(members),
            Task::Expr => self.expr(),
            Task::IfExpr => self.if_expr(),
            Task::IfThen => self.if_then(),
            Task::IfElse => self.if_else(),
            Task::OrExpr => self.or_expr(),
            Task::OrLoop(checkpoint) => self.or_loop(checkpoint),
            Task::AndExpr => self.and_expr(),
            Task::AndLoop(checkpoint) => self.and_loop(checkpoint),
            Task::NotExpr => self.not_expr(),
            Task::CmpExpr => self.cmp_expr(),
            Task::CmpLoop {
                checkpoint,
                chained,
            } => self.cmp_loop(checkpoint, chained),
            Task::AddExpr => self.add_expr(),
            Task::AddLoop(checkpoint) => self.add_loop(checkpoint),
            Task::MulExpr => self.mul_expr(),
            Task::MulLoop(checkpoint) => self.mul_loop(checkpoint),
            Task::UnaryExpr => self.unary_expr(),
            Task::PowExpr => self.pow_expr(),
            Task::PowTail(checkpoint) => self.pow_tail(checkpoint),
            Task::Postfix => self.postfix(),
            Task::Primary(checkpoint) => self.primary(checkpoint),
            Task::PostfixLoop(checkpoint) => self.postfix_loop(checkpoint),
            Task::ArrayLit => self.array_lit(),
            Task::ArrayLoop => self.array_loop(),
            Task::ArgList => self.arg_list(),
            Task::ArgItem { named_call } => self.arg_item(named_call),
            Task::ArgAfter { named_call } => self.arg_after(named_call),
            Task::StringLit => self.string_lit(),
            Task::StringLoop => self.string_loop(),
            Task::InterpEnd => self.interp_end(),
        }
    }

    fn significant(&self, n: usize) -> Option<usize> {
        let mut seen = 0;
        for index in self.pos..self.tokens.len() {
            if self.tokens[index].kind.is_trivia() {
                continue;
            }
            if seen == n {
                return Some(index);
            }
            seen += 1;
        }
        None
    }

    fn nth(&self, n: usize) -> Option<SyntaxKind> {
        self.significant(n).map(|index| self.tokens[index].kind)
    }

    fn peek(&self) -> Option<SyntaxKind> {
        self.nth(0)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.peek() == Some(kind)
    }

    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        self.peek().is_some_and(|kind| kinds.contains(&kind))
    }

    fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    fn adjacent(&self) -> bool {
        self.tokens
            .get(self.pos)
            .is_some_and(|token| !token.kind.is_trivia())
    }

    fn text_of(&self, index: usize) -> &'a str {
        let range = &self.tokens[index].range;
        &self.source.text[range.start as usize..range.end as usize]
    }

    fn current_text(&self) -> &'a str {
        self.significant(0).map_or("", |index| self.text_of(index))
    }

    fn at_wire_kind(&self) -> bool {
        self.at(SyntaxKind::Ident)
            && self
                .vocabulary
                .wire_kind_named(&nfc(self.current_text()))
                .is_some()
    }

    fn raw_bump(&mut self) {
        let token = &self.tokens[self.pos];
        let text = &self.source.text[token.range.start as usize..token.range.end as usize];
        self.builder.token(token.kind, text);
        self.pos += 1;
    }

    fn flush_trivia(&mut self) {
        while self
            .tokens
            .get(self.pos)
            .is_some_and(|token| token.kind.is_trivia())
        {
            self.raw_bump();
        }
    }

    fn bump(&mut self) {
        self.flush_trivia();
        if self.pos >= self.tokens.len() {
            return;
        }
        let kind = self.tokens[self.pos].kind;
        self.raw_bump();
        if kind == SyntaxKind::Newline {
            self.previous_line_error = self.line_error;
            self.line_error = false;
        }
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn start(&mut self, kind: SyntaxKind) {
        self.flush_trivia();
        self.builder.start_node(kind);
    }

    fn finish(&mut self) {
        self.builder.finish_node();
    }

    fn checkpoint(&mut self) -> Checkpoint {
        self.flush_trivia();
        self.builder.checkpoint()
    }

    fn start_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        self.builder.start_node_at(checkpoint, kind);
    }

    fn current_span(&self) -> Span {
        let range = match self.significant(0) {
            Some(index) => {
                let mut range = self.tokens[index].range.clone();
                if self.tokens[index].kind == SyntaxKind::Indent
                    && let Some(next) = self.tokens[index..]
                        .iter()
                        .find(|token| !token.range.is_empty() && !token.kind.is_trivia())
                {
                    range = next.range.clone();
                }
                range
            }
            None => {
                let end = self.source.text.len() as u32;
                end..end
            }
        };
        Span {
            file: self.source.path.clone(),
            range,
        }
    }

    fn error(&mut self, code: Code, message: impl Into<String>) {
        self.error_with(Diagnostic::new(code, self.current_span(), message));
    }

    fn error_with(&mut self, diagnostic: Diagnostic) {
        if self.line_error {
            return;
        }
        self.line_error = true;
        if !self.at(SyntaxKind::Error) && !self.lexer_error_on_line(diagnostic.span.range.start) {
            self.diagnostics.push(diagnostic);
        }
        self.resync();
    }

    fn lexer_error_on_line(&self, at: u32) -> bool {
        let text = &self.source.text;
        let at = (at as usize).min(text.len());
        let start = text[..at].rfind('\n').map_or(0, |index| index + 1);
        let end = text[at..].find('\n').map_or(text.len(), |index| at + index);
        self.lexer_diagnostics.iter().any(|diagnostic| {
            let position = diagnostic.span.range.start as usize;
            position >= start && position <= end
        })
    }

    pub(super) fn abandon_bracket(&mut self) {
        if !self.line_error {
            self.line_error = true;
            self.resync();
        }
    }

    fn resync(&mut self) {
        let Some(current) = self.significant(0) else {
            return;
        };
        if self.depths.get(current).copied().unwrap_or(0) == 0 {
            return;
        }
        let from = self.tokens[current].range.start as usize;
        let region_end = (current..self.tokens.len())
            .find(|index| self.depths[*index] == 0)
            .map_or(self.source.text.len(), |index| {
                self.tokens[index].range.start as usize
            });
        let Some(line) = self
            .suspended
            .iter()
            .find(|line| {
                line.first >= from
                    && line.first < region_end
                    && line.column <= line.stack.last().copied().unwrap_or(0)
            })
            .cloned()
        else {
            return;
        };
        let Some(cut) = (self.pos..self.tokens.len())
            .find(|index| self.tokens[*index].range.start as usize >= line.offset)
        else {
            return;
        };
        let (lexed, layout) =
            Lexer::resume(self.source, self.vocabulary, line.offset, line.stack).run_with_layout();
        self.tokens.truncate(cut);
        self.tokens.extend(lexed.tokens);
        self.depths.truncate(cut);
        self.depths.extend(layout.depths);
        self.suspended
            .retain(|suspension| suspension.offset < line.offset);
        self.suspended.extend(layout.suspended);
        self.lexer_diagnostics
            .retain(|diagnostic| (diagnostic.span.range.start as usize) < line.offset);
        self.lexer_diagnostics.extend(lexed.diagnostics);
    }

    fn describe(&self) -> String {
        match self.peek() {
            None => String::from("the end of the file"),
            Some(SyntaxKind::Newline) => String::from("the end of the line"),
            Some(SyntaxKind::Indent) => String::from("an indented line"),
            Some(SyntaxKind::Dedent) => String::from("the end of the block"),
            Some(_) => format!("`{}`", self.current_text()),
        }
    }

    fn expect(&mut self, kind: SyntaxKind, code: Code, what: &str) -> bool {
        if self.eat(kind) {
            return true;
        }
        let found = self.describe();
        self.error(code, format!("expected {what}, found {found}"));
        false
    }

    fn expect_name(&mut self, code: Code, what: &str) -> bool {
        self.expect(SyntaxKind::Ident, code, what)
    }

    fn skip_line(&mut self) {
        if self.at_any(&[SyntaxKind::Newline, SyntaxKind::Dedent]) || self.at_end() {
            return;
        }
        self.start(SyntaxKind::ErrorNode);
        while !self.at_end() && !self.at_any(&[SyntaxKind::Newline, SyntaxKind::Dedent]) {
            if self.at(SyntaxKind::Indent) {
                self.skip_block();
            } else {
                self.bump();
            }
        }
        self.finish();
    }

    fn skip_block(&mut self) {
        self.start(SyntaxKind::ErrorNode);
        let mut depth = 0usize;
        while let Some(kind) = self.peek() {
            match kind {
                SyntaxKind::Indent => depth += 1,
                SyntaxKind::Dedent => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.bump();
            if depth == 0 {
                break;
            }
        }
        self.finish();
    }

    fn finish_line(&mut self, code: Code) {
        if self.eat(SyntaxKind::Newline) || self.at_end() || self.at(SyntaxKind::Dedent) {
            return;
        }
        let found = self.describe();
        self.error(code, format!("expected the end of the line, found {found}"));
        self.skip_line();
        self.eat(SyntaxKind::Newline);
    }

    fn unexpected_indent(&mut self) {
        if !self.previous_line_error {
            let diagnostic = Diagnostic::new(
                Code::UnexpectedIndent,
                self.current_span(),
                "this line is indented, but the line before it opens no block",
            );
            self.diagnostics.push(diagnostic);
        }
        self.skip_block();
    }

    fn skip_token(&mut self) {
        self.start(SyntaxKind::ErrorNode);
        self.bump();
        self.finish();
    }
}
