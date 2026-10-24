mod blocks;
mod expressions;
mod items;
mod placements;

use rowan::{Checkpoint, GreenNode, GreenNodeBuilder, Language};
use symphonyscript_core::{Span, Vocabulary};

use super::nfc::nfc;
use super::symphony_language::SymphonyLanguage;
use super::syntax_kind::SyntaxKind;
use super::token::Token;
use crate::diagnostics::{Code, Diagnostic};
use crate::source::Source;

pub(crate) struct Parser<'a> {
    source: &'a Source,
    vocabulary: &'a Vocabulary,
    tokens: Vec<Token>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    diagnostics: Vec<Diagnostic>,
    line_error: bool,
    previous_line_error: bool,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(source: &'a Source, vocabulary: &'a Vocabulary, tokens: Vec<Token>) -> Self {
        Self {
            source,
            vocabulary,
            tokens,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            diagnostics: Vec::new(),
            line_error: false,
            previous_line_error: false,
        }
    }

    pub(crate) fn run(mut self) -> (GreenNode, Vec<Diagnostic>) {
        self.builder
            .start_node(SymphonyLanguage::kind_to_raw(SyntaxKind::File));
        self.file();
        self.flush_trivia();
        while self.pos < self.tokens.len() {
            self.raw_bump();
        }
        self.builder.finish_node();
        (self.builder.finish(), self.diagnostics)
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
        self.builder
            .token(SymphonyLanguage::kind_to_raw(token.kind), text);
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
        self.builder.start_node(SymphonyLanguage::kind_to_raw(kind));
    }

    fn finish(&mut self) {
        self.builder.finish_node();
    }

    fn checkpoint(&mut self) -> Checkpoint {
        self.flush_trivia();
        self.builder.checkpoint()
    }

    fn start_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        self.builder
            .start_node_at(checkpoint, SymphonyLanguage::kind_to_raw(kind));
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
        if self.at(SyntaxKind::Error) {
            return;
        }
        self.diagnostics.push(diagnostic);
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
