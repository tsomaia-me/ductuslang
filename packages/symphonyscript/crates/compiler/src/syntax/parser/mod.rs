mod blocks;
mod bracket;
mod checkpoint;
mod cost;
mod expressions;
mod items;
mod placements;
mod starts_line;
mod task;
#[cfg(test)]
mod tests;
mod tree_builder;

use std::ops::Range;

use rowan::GreenNode;
use symphonyscript_core::{Span, Vocabulary};

use self::bracket::Bracket;
use self::checkpoint::Checkpoint;
use self::cost::Cost;
use self::starts_line::starts_line;
use self::task::Task;
use self::tree_builder::TreeBuilder;
use super::lexer::{Lexer, LineStart};
use super::nfc::nfc;
use super::syntax_kind::SyntaxKind;
use crate::diagnostics::{Code, Diagnostic};
use crate::source::Source;

pub(crate) struct Parser<'a> {
    source: &'a Source,
    vocabulary: &'a Vocabulary,
    lexer: Lexer<'a>,
    pos: usize,
    brackets: Vec<Bracket>,
    builder: TreeBuilder,
    diagnostics: Vec<Diagnostic>,
    detached: Vec<Range<u32>>,
    line_error: bool,
    previous_line_error: bool,
    tasks: Vec<Task>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(source: &'a Source, vocabulary: &'a Vocabulary) -> Self {
        Self {
            source,
            vocabulary,
            lexer: Lexer::new(source, vocabulary),
            pos: 0,
            brackets: Vec::new(),
            builder: TreeBuilder::new(),
            diagnostics: Vec::new(),
            detached: Vec::new(),
            line_error: false,
            previous_line_error: false,
            tasks: Vec::new(),
        }
    }

    pub(crate) fn run(self) -> (GreenNode, Vec<Diagnostic>, Vec<Range<u32>>) {
        let (green, diagnostics, _, detached) = self.parsed();
        (green, diagnostics, detached)
    }

    #[cfg(test)]
    fn run_counting(self) -> (GreenNode, Vec<Diagnostic>, Cost) {
        let (green, diagnostics, cost, _) = self.parsed();
        (green, diagnostics, cost)
    }

    fn parsed(mut self) -> (GreenNode, Vec<Diagnostic>, Cost, Vec<Range<u32>>) {
        self.builder.start_node(SyntaxKind::File);
        self.tasks.push(Task::File);
        while let Some(task) = self.tasks.pop() {
            self.step(task);
        }
        self.flush_trivia();
        while self.fill(self.pos) {
            self.raw_bump();
        }
        self.builder.finish_node();
        let green = self.builder.finish();
        let cost = Cost {
            work: self.lexer.work(),
            peak_stored_entries: self.lexer.peak_stored_entries(),
        };
        let mut diagnostics = self.lexer.into_diagnostics();
        diagnostics.extend(self.diagnostics);
        diagnostics.sort_by_key(|diagnostic| diagnostic.span.range.start);
        (green, diagnostics, cost, self.detached)
    }

    fn fill(&mut self, index: usize) -> bool {
        while self.lexer.settled() <= index {
            if !self.lexer.advance() {
                return false;
            }
        }
        true
    }

    fn kind_at(&mut self, index: usize) -> Option<SyntaxKind> {
        if !self.fill(index) {
            return None;
        }
        self.lexer.token_at(index).map(|token| token.kind)
    }

    fn range_at(&mut self, index: usize) -> Option<Range<u32>> {
        if !self.fill(index) {
            return None;
        }
        self.lexer.token_at(index).map(|token| token.range.clone())
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
            Task::FnColon => self.fn_colon(),
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
            Task::DefaultArg { kind } => self.default_arg(kind),
            Task::DefaultAfterAtom { kind, slash, atom } => {
                self.default_after_atom(kind, slash, atom)
            }
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
            Task::IndexBody => self.index_body(),
            Task::IndexAfterBound => self.index_after_bound(),
            Task::SliceUpper => self.slice_upper(),
            Task::Bound => self.bound(),
            Task::HistoryAfterOffset => self.history_after_offset(),
            Task::ArgList => self.arg_list(),
            Task::ArgItem { named_call } => self.arg_item(named_call),
            Task::ArgAfter { named_call } => self.arg_after(named_call),
            Task::StringLit => self.string_lit(),
            Task::StringLoop => self.string_loop(),
            Task::InterpEnd => self.interp_end(),
        }
    }

    fn significant(&mut self, n: usize) -> Option<usize> {
        let mut seen = 0;
        let mut index = self.pos;
        while let Some(kind) = self.kind_at(index) {
            if !kind.is_trivia() {
                if seen == 0 && self.end_region_before(index) {
                    index = self.pos;
                    continue;
                }
                if seen == n {
                    return Some(index);
                }
                seen += 1;
            }
            index += 1;
        }
        None
    }

    fn end_region_before(&mut self, index: usize) -> bool {
        let Some(bracket) = self.brackets.last().copied() else {
            return false;
        };
        if bracket == Bracket::Other {
            return false;
        }
        let Some((line, start)) = self.lexer.suspended_line_at(index) else {
            return false;
        };
        let ends = match bracket {
            Bracket::Expression => {
                start.first.kind == SyntaxKind::Ident
                    && start
                        .rest
                        .iter()
                        .find(|token| !token.kind.is_trivia())
                        .is_some_and(|next| {
                            matches!(next.kind, SyntaxKind::Ident | SyntaxKind::AsKw)
                        })
            }
            Bracket::Bundle { depth, header } => holds_stray_colon(&start, depth, header),
            Bracket::Other => false,
        };
        if ends {
            self.lexer.resynchronize(line, self.pos);
        }
        ends
    }

    fn nth(&mut self, n: usize) -> Option<SyntaxKind> {
        let index = self.significant(n)?;
        self.kind_at(index)
    }

    fn peek(&mut self) -> Option<SyntaxKind> {
        self.nth(0)
    }

    fn at(&mut self, kind: SyntaxKind) -> bool {
        self.peek() == Some(kind)
    }

    fn at_any(&mut self, kinds: &[SyntaxKind]) -> bool {
        self.peek().is_some_and(|kind| kinds.contains(&kind))
    }

    fn at_end(&mut self) -> bool {
        self.peek().is_none()
    }

    fn adjacent(&mut self) -> bool {
        self.kind_at(self.pos).is_some_and(|kind| !kind.is_trivia())
    }

    fn text_of(&mut self, index: usize) -> &'a str {
        let source = self.source;
        self.range_at(index)
            .and_then(|range| source.text.get(range.start as usize..range.end as usize))
            .unwrap_or("")
    }

    fn text_between(&mut self, first: usize, last: usize) -> &'a str {
        let source = self.source;
        let start = self.range_at(first).map_or(0, |range| range.start);
        let end = self.range_at(last).map_or(start, |range| range.end);
        source.text.get(start as usize..end as usize).unwrap_or("")
    }

    fn closed_groups(&mut self) -> Option<usize> {
        let source = self.source;
        let open = self.significant(0)?;
        let open_start = self.range_at(open)?.start as usize;
        let line_end = source
            .text
            .get(open_start..)
            .and_then(|rest| rest.find('\n'))
            .map_or(source.text.len(), |offset| open_start + offset);
        let mut openers: Vec<SyntaxKind> = Vec::new();
        let mut index = open;
        loop {
            let kind = self.kind_at(index)?;
            if self.range_at(index)?.start as usize >= line_end {
                return None;
            }
            match kind {
                SyntaxKind::LParen | SyntaxKind::LBracket => openers.push(kind),
                SyntaxKind::RParen if openers.pop() != Some(SyntaxKind::LParen) => return None,
                SyntaxKind::RBracket if openers.pop() != Some(SyntaxKind::LBracket) => {
                    return None;
                }
                _ => {}
            }
            if openers.is_empty() {
                while self.kind_at(index + 1) == Some(SyntaxKind::Dot)
                    && self.kind_at(index + 2) == Some(SyntaxKind::Ident)
                {
                    index += 2;
                }
                if !matches!(
                    self.kind_at(index + 1),
                    Some(SyntaxKind::LParen | SyntaxKind::LBracket)
                ) {
                    return Some(index);
                }
            }
            index += 1;
        }
    }

    fn current_text(&mut self) -> &'a str {
        match self.significant(0) {
            Some(index) => self.text_of(index),
            None => "",
        }
    }

    fn at_wire_kind(&mut self) -> bool {
        self.at(SyntaxKind::Ident)
            && self
                .vocabulary
                .wire_kind_named(&nfc(self.current_text()))
                .is_some()
    }

    fn raw_bump(&mut self) {
        let Some(kind) = self.kind_at(self.pos) else {
            return;
        };
        match kind {
            SyntaxKind::LParen
            | SyntaxKind::LBracket
            | SyntaxKind::LBrace
            | SyntaxKind::InterpStart => {
                let bracket = match self.builder.open_kind() {
                    Some(
                        SyntaxKind::ArrayLit
                        | SyntaxKind::ParenExpr
                        | SyntaxKind::ArgList
                        | SyntaxKind::IndexExpr
                        | SyntaxKind::Conversion
                        | SyntaxKind::Interpolation,
                    ) => Bracket::Expression,
                    Some(SyntaxKind::Bundle) => Bracket::Bundle {
                        depth: self.lexer.depth_at(self.pos).unwrap_or(0) + 1,
                        header: false,
                    },
                    _ => Bracket::Other,
                };
                self.brackets.push(bracket);
            }
            SyntaxKind::RParen
            | SyntaxKind::RBracket
            | SyntaxKind::RBrace
            | SyntaxKind::InterpEnd => {
                self.brackets.pop();
            }
            SyntaxKind::ForKw | SyntaxKind::Colon => {
                if let Some(Bracket::Bundle { header, .. }) = self.brackets.last_mut() {
                    *header = kind == SyntaxKind::ForKw;
                }
            }
            SyntaxKind::Newline => self.brackets.clear(),
            _ => {}
        }
        let opens_region = matches!(
            kind,
            SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::LBrace
        ) && self.line_error
            && self.lexer.depth_at(self.pos) == Some(0);
        let text = self.text_of(self.pos);
        self.builder.token(kind, text);
        self.pos += 1;
        if opens_region {
            self.resync_at(self.pos - 1);
        }
    }

    fn flush_trivia(&mut self) {
        while self.kind_at(self.pos).is_some_and(SyntaxKind::is_trivia) {
            self.raw_bump();
        }
    }

    fn bump(&mut self) {
        self.flush_trivia();
        let Some(kind) = self.kind_at(self.pos) else {
            return;
        };
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

    fn current_span(&mut self) -> Span {
        let range = match self.significant(0) {
            Some(index) => self.visible_range(index),
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

    fn visible_range(&mut self, index: usize) -> Range<u32> {
        let range = self.range_at(index).unwrap_or_default();
        let layout = match self.kind_at(index) {
            Some(SyntaxKind::Indent) => true,
            Some(SyntaxKind::Newline) => range.is_empty(),
            _ => false,
        };
        if !layout {
            return range;
        }
        let mut next = index;
        while let Some(kind) = self.kind_at(next) {
            let candidate = self.range_at(next).unwrap_or_default();
            if !candidate.is_empty() && !kind.is_trivia() {
                return candidate;
            }
            next += 1;
        }
        range
    }

    fn error(&mut self, code: Code, message: impl Into<String>) {
        let span = self.current_span();
        let diagnostic = if self.at(SyntaxKind::Caret) {
            Diagnostic::new(
                Code::MisplacedCaret,
                span,
                "`^` counts from an array's end and stands only at the start of an index or a slice bound",
            )
        } else {
            Diagnostic::new(code, span, message)
        };
        self.error_with(diagnostic);
    }

    fn missing_block(&mut self, code: Code, message: impl Into<String>) {
        if self.line_error {
            return;
        }
        let diagnostic = Diagnostic::new(code, self.current_span(), message);
        self.detached.push(diagnostic.span.range.clone());
        self.diagnostics.push(diagnostic);
    }

    fn error_with(&mut self, diagnostic: Diagnostic) {
        if self.line_error {
            return;
        }
        self.line_error = true;
        if !self.at(SyntaxKind::Error) && !self.same_fault_as_lexer(diagnostic.span.range.start) {
            self.diagnostics.push(diagnostic);
        }
        self.resync();
    }

    fn same_fault_as_lexer(&mut self, at: u32) -> bool {
        if self.lexer.faulted_at(at) {
            return true;
        }
        let Some(before) = self.lexer.token_before(at) else {
            return false;
        };
        if self.kind_at(before) == Some(SyntaxKind::Error) {
            return true;
        }
        let Some(range) = self.range_at(before) else {
            return false;
        };
        self.lexer.in_cut_construct(range.start)
    }

    pub(super) fn abandon_bracket(&mut self) {
        if !self.line_error {
            self.line_error = true;
            self.resync();
        }
    }

    fn resync(&mut self) {
        if let Some(current) = self.significant(0) {
            self.resync_at(current);
        }
    }

    fn resync_at(&mut self, current: usize) {
        let (text, vocabulary) = (self.source.text.as_str(), self.vocabulary);
        let starts = |line: &LineStart<'_>| starts_line(line, text, vocabulary);
        if let Some(line) = self.lexer.resync_line(current, starts) {
            self.lexer.resynchronize(line, self.pos);
        }
    }

    fn describe(&mut self) -> String {
        match self.peek() {
            None => String::from("the end of the file"),
            Some(SyntaxKind::Newline) => {
                let range = self.current_span().range;
                let source = self.source;
                match source.text.get(range.start as usize..range.end as usize) {
                    Some(text) if !text.trim().is_empty() => format!("`{text}`"),
                    _ => String::from("the end of the line"),
                }
            }
            Some(SyntaxKind::Indent) => String::from("an indented line"),
            Some(SyntaxKind::Dedent) => String::from("the end of the block"),
            Some(_) => {
                let text = self.current_text();
                format!("`{text}`")
            }
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

    fn colon_or_skip(&mut self, code: Code, what: &str) -> bool {
        if self.at(SyntaxKind::Colon) {
            return true;
        }
        let found = self.describe();
        self.error(code, format!("expected {what}, found {found}"));
        if self.skip_to_final_colon() {
            return true;
        }
        self.eat(SyntaxKind::Newline);
        false
    }

    fn match_after_fault(&mut self, anchor: SyntaxKind, code: Code, what: &str) -> bool {
        let found = self.describe();
        self.error(code, format!("expected {what}, found {found}"));
        let Some(index) = self.match_anchor(anchor) else {
            return false;
        };
        self.skip_to(index);
        if self.significant(0) == Some(index) && self.at(anchor) {
            self.bump();
            return true;
        }
        false
    }

    fn match_anchor(&mut self, anchor: SyntaxKind) -> Option<usize> {
        self.final_colon()?;
        let mut index = self.significant(0)?;
        loop {
            let kind = self.kind_at(index)?;
            if kind == SyntaxKind::Newline {
                return None;
            }
            if kind == anchor && self.lexer.depth_at(index) == Some(0) {
                let mut next = index + 1;
                while self.kind_at(next).is_some_and(SyntaxKind::is_trivia) {
                    next += 1;
                }
                if self.kind_at(next) == Some(SyntaxKind::MatchKw) {
                    return Some(index);
                }
            }
            index += 1;
        }
    }

    fn skip_to(&mut self, index: usize) {
        if self.significant(0).is_some_and(|current| current >= index) {
            return;
        }
        self.start(SyntaxKind::ErrorNode);
        while self.significant(0).is_some_and(|current| current < index)
            && !self.at_any(&[SyntaxKind::Newline, SyntaxKind::Dedent])
        {
            self.bump();
        }
        self.finish();
    }

    fn skip_to_final_colon(&mut self) -> bool {
        let Some(colon) = self.final_colon() else {
            self.skip_line();
            return false;
        };
        self.skip_to(colon);
        if self.significant(0) == Some(colon) && self.at(SyntaxKind::Colon) {
            return true;
        }
        self.skip_line();
        false
    }

    fn final_colon(&mut self) -> Option<usize> {
        let mut index = self.significant(0)?;
        let mut last = None;
        loop {
            match self.kind_at(index)? {
                SyntaxKind::Newline => break,
                SyntaxKind::Indent | SyntaxKind::Dedent => return None,
                kind if !kind.is_trivia() => last = Some(index),
                _ => {}
            }
            index += 1;
        }
        let colon = last.filter(|last| self.kind_at(*last) == Some(SyntaxKind::Colon))?;
        let mut next = index + 1;
        while self.kind_at(next).is_some_and(SyntaxKind::is_trivia) {
            next += 1;
        }
        (self.kind_at(next) == Some(SyntaxKind::Indent)).then_some(colon)
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

fn holds_stray_colon(line: &LineStart<'_>, depth: u32, mut header: bool) -> bool {
    let first = std::iter::once((line.first, &line.first_depth));
    for (token, token_depth) in first.chain(line.rest.iter().zip(line.rest_depths)) {
        if *token_depth != depth {
            continue;
        }
        match token.kind {
            SyntaxKind::ForKw => header = true,
            SyntaxKind::Colon if header => header = false,
            SyntaxKind::Colon => return true,
            _ => {}
        }
    }
    false
}
