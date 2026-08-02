//! [`parse()`] — the recursive-descent parser: one method per IR_GRAMMAR
//! production (R53).

use std::mem;

use crate::diag::{DiagClass, Diagnostic, Span};
use crate::module::{BehaviorsSection, GraphSection, Module, TypesSection};
use crate::text::token::{Token, TokenKind};

/// The soft keywords the module-level productions classify against (R46).
///
/// Every word-shaped token arrives as `Ident`, so these are compared as TEXT at
/// the position that expects them — a cell legally named `graph` stays a name
/// everywhere a keyword is not called for. Each spelling lives in exactly one
/// place, and a typo in one fails every parse test on real `.duir` text.
const MODULE: &str = "module";
const TYPES: &str = "types";
const GRAPH: &str = "graph";
const BEHAVIORS: &str = "behaviors";

/// Parse a `.duir` token stream into its [`Module`] (IR_GRAMMAR §3, §15.4.6).
///
/// Recursive descent with ONE private method per grammar production, named
/// after it 1:1 (R53): the grammar is this file's table of contents, and a
/// production that gains a clause is found under its own name.
///
/// Fail-fast (T6): the first token the grammar does not admit ends the parse
/// with an `unexpected_token` `Diagnostic` carrying that token's span. No
/// recovery, no error list — `.duir` is machine-written text.
///
/// The parser is a pure grammar machine (R21). It answers "is this the text
/// form?" and nothing else; duplicate paths, unknown references and slice
/// checks are the loader's (F4), so a `Module` that parses may still be an
/// illegal program.
///
/// **The stream is consumed, not borrowed.** R18 gives tokens OWNED payloads so
/// nothing downstream carries a lifetime; taking the `Vec` by value lets those
/// payloads MOVE lexer → parser → AST. A module's name is taken out of its
/// token rather than cloned out of it, and every construct row from increment 5
/// on inherits that shape — the text of a `.duir` file is allocated once.
///
/// **Newlines are optional at module level, and legal only BETWEEN
/// constructs.** 033-250 makes NEWLINE an entry terminator INSIDE the `types`,
/// `graph` and `behaviors` sections, and those entries arrive at increments
/// 5–7. Between the module-level brackets there is no entry to terminate, so a
/// newline is accepted at exactly these positions, and required at none of
/// them: before `module`, after the module's `{`, between sections, inside a
/// section's braces, before the module's `}`, and after it.
///
/// Everywhere else a newline is an `unexpected_token` — inside the module
/// header (`module\nApp`, `module App\n{`) and between a section keyword and
/// its brace (`types\n{`). Nothing skips newlines at those positions, and
/// nothing should: a construct's own first line is not a place to break.
///
/// **Every section is empty in this increment.** `types {}`, `graph {}` and
/// `behaviors {}` parse; anything else between a section's braces is an
/// `unexpected_token` until that section's row admits its entries.
pub fn parse(tokens: Vec<Token>) -> Result<Module, Diagnostic> {
    let mut parser = Parser { tokens, cursor: 0 };
    let module = parser.module()?;

    // The module is the whole file: past its closing brace only a line ending
    // may follow.
    parser.skip_newlines();
    parser.expect(&TokenKind::Eof, "end of input")?;

    Ok(module)
}

/// The stream and where we are in it — everything a production method needs
/// beyond the grammar itself.
struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    // module ::= 'module' NAME '{' types_section graph_section behaviors_section '}'
    fn module(&mut self) -> Result<Module, Diagnostic> {
        self.skip_newlines();

        let start = self.expect_word(MODULE)?;
        let name = self.name("a module name")?;
        self.expect(&TokenKind::LBrace, "`{`")?;

        let types = self.types_section()?;
        let graph = self.graph_section()?;
        let behaviors = self.behaviors_section()?;

        self.skip_newlines();
        let end = self.expect(&TokenKind::RBrace, "`}`")?;

        Ok(Module {
            name,
            span: covering(start, end),
            types,
            graph,
            behaviors,
        })
    }

    // types_section ::= 'types' '{' type_def* '}'
    fn types_section(&mut self) -> Result<TypesSection, Diagnostic> {
        self.skip_newlines();

        let start = self.expect_word(TYPES)?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        self.skip_newlines();
        let end = self.expect(
            &TokenKind::RBrace,
            "`}` (the `types` section admits no entries yet)",
        )?;

        Ok(TypesSection {
            span: covering(start, end),
        })
    }

    // graph_section ::= 'graph' '{' scope+ '}'
    //
    // The production's `scope+` arity is not enforced here: no scope form
    // exists before increment 7, so the empty section is all this increment can
    // admit.
    fn graph_section(&mut self) -> Result<GraphSection, Diagnostic> {
        self.skip_newlines();

        let start = self.expect_word(GRAPH)?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        self.skip_newlines();
        let end = self.expect(
            &TokenKind::RBrace,
            "`}` (the `graph` section admits no scopes yet)",
        )?;

        Ok(GraphSection {
            span: covering(start, end),
        })
    }

    // behaviors_section ::= 'behaviors' '{' behavior* '}'
    fn behaviors_section(&mut self) -> Result<BehaviorsSection, Diagnostic> {
        self.skip_newlines();

        let start = self.expect_word(BEHAVIORS)?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        self.skip_newlines();
        let end = self.expect(
            &TokenKind::RBrace,
            "`}` (the `behaviors` section admits no behaviors yet)",
        )?;

        Ok(BehaviorsSection {
            span: covering(start, end),
        })
    }

    // What the token at the cursor IS, or `None` past the end of the stream.
    // `lex` always ends a stream in `Eof`, so `None` is reachable only from a
    // hand-built slice — but a parser that panics on one is worse than a parser
    // that reports `unexpected_token`.
    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.cursor).map(|token| &token.kind)
    }

    // Where a diagnostic about the cursor points: the token's own span, or —
    // past the end of the stream — a ZERO-LENGTH span at the last real token's
    // end (offset 0 for an empty stream). Pointing just past what the caller
    // handed in beats pointing at byte 0 of a file the error has nothing to do
    // with.
    fn span(&self) -> Span {
        match self.tokens.get(self.cursor) {
            Some(token) => token.span,
            None => Span::new(self.tokens.last().map_or(0, |token| token.span.end()), 0),
        }
    }

    // Consume the token at the cursor and report its span.
    fn advance(&mut self) -> Span {
        let span = self.span();
        self.cursor += 1;

        span
    }

    // Soft-keyword classification (R46): the token must be an `Ident` whose
    // text is exactly `word`. The lexer knows no keywords, so this — a
    // position expecting a spelling — is the only place a word becomes one.
    // The comparison reads the payload in place and allocates nothing.
    fn expect_word(&mut self, word: &str) -> Result<Span, Diagnostic> {
        if matches!(self.peek_kind(), Some(TokenKind::Ident(text)) if text == word) {
            return Ok(self.advance());
        }

        Err(self.unexpected(&format!("`{word}`")))
    }

    // The NAME lexeme (§2): any word-shaped token, taken for its text —
    // literally TAKEN (R18). The payload moves out of the token and into the
    // AST; the emptied token is consumed on the next line and never read again,
    // so the name exists in exactly one place from here on.
    fn name(&mut self, expected: &str) -> Result<String, Diagnostic> {
        match self.tokens.get_mut(self.cursor) {
            Some(Token {
                kind: TokenKind::Ident(text),
                ..
            }) => {
                let name = mem::take(text);
                self.advance();

                Ok(name)
            }
            _ => Err(self.unexpected(expected)),
        }
    }

    // A token of the same KIND as `kind`, payload ignored. Every caller here
    // passes a payload-free mark; comparing discriminants rather than whole
    // values is what keeps a later `expect(&TokenKind::Int(…))` from matching
    // one literal value instead of the lexeme class.
    fn expect(&mut self, kind: &TokenKind, expected: &str) -> Result<Span, Diagnostic> {
        let matched = self
            .peek_kind()
            .is_some_and(|found| mem::discriminant(found) == mem::discriminant(kind));

        if matched {
            return Ok(self.advance());
        }

        Err(self.unexpected(expected))
    }

    // A run of newlines collapses to one token in the lexer (R48), so the loop
    // runs at most once on a lexed stream and stays correct on any other.
    //
    // WARNING — increment 5's entry loops must NOT copy this leading-skip
    // convention. 033-250 makes NEWLINE a REQUIRED entry terminator inside the
    // `types`, `graph` and `behaviors` sections; a loop that opens by skipping
    // newlines silently accepts two entries written on one line. The required-
    // terminator helper is a different helper and lands with the entries it
    // binds, at increment 5. Module level may skip freely for one reason only:
    // between the module-level brackets there is no entry to terminate.
    fn skip_newlines(&mut self) {
        while self.peek_kind() == Some(&TokenKind::Newline) {
            self.advance();
        }
    }

    // The offending token is the one AT the cursor: it is what the grammar
    // rejected, and its span is where a reader must look.
    fn unexpected(&self, expected: &str) -> Diagnostic {
        Diagnostic {
            class: DiagClass::UnexpectedToken,
            span: Some(self.span()),
            path: None,
            message: format!("expected {expected}, found {}", describe(self.peek_kind())),
        }
    }
}

// The span from `start`'s first byte through `end`'s last — the one place that
// arithmetic is written.
//
// Saturating, so a hand-built stream whose spans DESCEND yields a degenerate
// zero-length span rather than panicking in debug or wrapping to an enormous
// length in release. Spans from `lex` are monotonic by construction; a span is
// a diagnostic aid, and no aid is worth a crash.
fn covering(start: Span, end: Span) -> Span {
    Span::new(start.start, end.end().saturating_sub(start.start))
}

// How the offending token reads in a diagnostic: its literal spelling where it
// HAS one, its class name where the payload does not belong in a message. Past
// the end of the stream reads the same as an explicit `Eof`.
//
// The match is exhaustive on purpose: a token variant added by a later row has
// to be given a rendering here before it compiles.
fn describe(kind: Option<&TokenKind>) -> String {
    let Some(kind) = kind else {
        return "end of input".to_string();
    };

    let fixed = match kind {
        TokenKind::Ident(text) | TokenKind::Int(text) | TokenKind::Float(text) => {
            return format!("`{text}`");
        }
        TokenKind::Str(_) => "a string",
        TokenKind::QuotedKey(_) => "a quoted key",
        TokenKind::Bid(_) => "a behavior handle",
        TokenKind::LBrace => "`{`",
        TokenKind::RBrace => "`}`",
        TokenKind::LBracket => "`[`",
        TokenKind::RBracket => "`]`",
        TokenKind::LParen => "`(`",
        TokenKind::RParen => "`)`",
        TokenKind::Lt => "`<`",
        TokenKind::Gt => "`>`",
        TokenKind::Colon => "`:`",
        TokenKind::Comma => "`,`",
        TokenKind::Dot => "`.`",
        TokenKind::Semicolon => "`;`",
        TokenKind::Eq => "`=`",
        TokenKind::Percent => "`%`",
        TokenKind::Hash => "`#`",
        TokenKind::Arrow => "`->`",
        TokenKind::At => "`@`",
        TokenKind::Newline => "a line ending",
        TokenKind::Eof => "end of input",
    };

    fixed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::emitter::emit;
    use crate::text::lexer::lex;

    // Byte offsets, counted by hand: `module` 0..6, `App` 7..10, `{` 11,
    // `types` 15..20, `{}` 21..23, `graph` 26..31, `{}` 32..34,
    // `behaviors` 37..46, `{}` 47..49, `}` 50, trailing newline 51, end 52.
    const CANONICAL: &str = "module App {\n  types {}\n  graph {}\n  behaviors {}\n}\n";

    // The same module with no newline anywhere, not even a trailing one.
    const ONE_LINE: &str = "module App { types {} graph {} behaviors {} }";

    // The same module again, blank lines everywhere they are legal.
    const BLANK_LINES: &str =
        "\n\nmodule App {\n\n  types {}\n\n\n  graph {}\n\n  behaviors {}\n\n}\n\n";

    fn parsed(source: &str) -> Module {
        let tokens = lex(source).expect("source lexes");

        parse(tokens).expect("source parses")
    }

    // Class and span only — never the message (F3).
    fn failure(source: &str) -> (&'static str, Span) {
        let tokens = lex(source).expect("source lexes");
        let diagnostic = parse(tokens).expect_err("source fails to parse");

        (
            diagnostic.class.id(),
            diagnostic
                .span
                .expect("a parse error always carries a span"),
        )
    }

    #[test]
    fn a_minimal_module_parses_with_hand_counted_spans() {
        assert_eq!(CANONICAL.len(), 52);

        let module = parsed(CANONICAL);

        assert_eq!(module.name, "App");
        // `module` through the final `}`, trailing newline excluded.
        assert_eq!(module.span, Span::new(0, 51));
        assert_eq!(module.types.span, Span::new(15, 8));
        assert_eq!(module.graph.span, Span::new(26, 8));
        assert_eq!(module.behaviors.span, Span::new(37, 12));
    }

    // Structural sameness is emitted-text equality (R17): the AST derives no
    // equality, and spans differ between these sources by construction.
    #[test]
    fn newlines_are_optional_between_module_level_brackets() {
        assert_eq!(emit(&parsed(ONE_LINE)), emit(&parsed(CANONICAL)));
        assert_eq!(emit(&parsed(BLANK_LINES)), emit(&parsed(CANONICAL)));
        // Including inside an empty section's braces.
        assert_eq!(
            emit(&parsed(
                "module App {\n  types {\n  }\n  graph {}\n  behaviors {}\n}\n"
            )),
            emit(&parsed(CANONICAL))
        );
    }

    #[test]
    fn a_leading_newline_is_accepted() {
        assert_eq!(ONE_LINE.len(), 45);

        let module = parsed(&format!("\n{ONE_LINE}"));

        assert_eq!(module.name, "App");
        // Everything shifts one byte: `module` at 1, the final `}` at 45.
        assert_eq!(module.span, Span::new(1, 45));
    }

    // The other half of the newline law: legal BETWEEN constructs, never inside
    // one. Nothing skips newlines at these three positions.
    #[test]
    fn a_newline_inside_a_construct_is_rejected() {
        // Between `module` and the name.
        assert_eq!(
            failure("module\nApp { types {} graph {} behaviors {} }"),
            ("unexpected_token", Span::new(6, 1))
        );
        // Between the name and the module's `{`.
        assert_eq!(
            failure("module App\n{ types {} graph {} behaviors {} }"),
            ("unexpected_token", Span::new(10, 1))
        );
        // Between a section keyword and its `{`.
        assert_eq!(
            failure("module App { types\n{} graph {} behaviors {} }"),
            ("unexpected_token", Span::new(18, 1))
        );
    }

    // R46's whole point, pinned: the lexer knows no keywords, so a section
    // spelling is a perfectly good module name. The keyword position and the
    // name position are different positions.
    #[test]
    fn a_section_keyword_is_a_legal_module_name() {
        for name in [TYPES, GRAPH, BEHAVIORS, MODULE] {
            let module = parsed(&format!(
                "module {name} {{ types {{}} graph {{}} behaviors {{}} }}"
            ));
            let canonical =
                format!("module {name} {{\n  types {{}}\n  graph {{}}\n  behaviors {{}}\n}}\n");

            assert_eq!(module.name, name);
            assert_eq!(emit(&module), canonical);
            // And the canonical text parses back to itself.
            assert_eq!(emit(&parsed(&canonical)), canonical);
        }
    }

    #[test]
    fn a_missing_module_header_is_unexpected_at_its_first_token() {
        assert_eq!(
            failure("App { types {} graph {} behaviors {} }"),
            ("unexpected_token", Span::new(0, 3))
        );
        assert_eq!(
            failure("module { types {} graph {} behaviors {} }"),
            ("unexpected_token", Span::new(7, 1))
        );

        // An empty stream is not a lexer product, but it must not panic.
        let diagnostic = parse(Vec::new()).expect_err("an empty stream fails to parse");

        assert_eq!(diagnostic.class.id(), "unexpected_token");
        assert_eq!(diagnostic.span, Some(Span::new(0, 0)));
    }

    #[test]
    fn sections_are_required_in_grammar_order() {
        // `graph` standing where `types` belongs.
        assert_eq!(
            failure("module App {\n  graph {}\n  types {}\n  behaviors {}\n}\n"),
            ("unexpected_token", Span::new(15, 5))
        );
        // `graph` missing entirely: `behaviors` is what turns up in its place.
        assert_eq!(
            failure("module App {\n  types {}\n  behaviors {}\n}\n"),
            ("unexpected_token", Span::new(26, 9))
        );
    }

    #[test]
    fn the_module_must_close_and_nothing_may_follow_it() {
        // No closing `}`: the error lands on `Eof`'s zero-length span at 50.
        assert_eq!(
            failure("module App {\n  types {}\n  graph {}\n  behaviors {}\n"),
            ("unexpected_token", Span::new(50, 0))
        );

        let trailing = format!("{CANONICAL}x");

        assert_eq!(failure(&trailing), ("unexpected_token", Span::new(52, 1)));
    }

    // A stream that simply STOPS — no `Eof`, which `lex` always supplies. The
    // diagnostic points just past the last token that does exist, not at byte 0.
    #[test]
    fn a_truncated_stream_reports_at_the_end_of_its_last_token() {
        let mut tokens = lex(CANONICAL).expect("source lexes");
        // Drop `Eof` at 52, the trailing newline at 51, and the module's `}` at
        // 50, leaving the newline at 49 as the last token.
        tokens.truncate(tokens.len() - 3);

        let diagnostic = parse(tokens).expect_err("a truncated stream fails to parse");

        assert_eq!(diagnostic.class.id(), "unexpected_token");
        assert_eq!(diagnostic.span, Some(Span::new(50, 0)));
    }

    // `covering` subtracts one span's start from another's end. A hand-built
    // stream whose spans descend makes that difference negative: without the
    // saturating guard this panics in debug and wraps in release. The stream
    // also runs out early, so the parse must end the way any short stream does.
    #[test]
    fn a_descending_span_stream_degrades_instead_of_underflowing() {
        let tokens = vec![
            Token::new(TokenKind::Ident(MODULE.to_string()), Span::new(50, 6)),
            Token::new(TokenKind::Ident("App".to_string()), Span::new(40, 3)),
            Token::new(TokenKind::LBrace, Span::new(30, 1)),
            Token::new(TokenKind::Ident(TYPES.to_string()), Span::new(20, 5)),
            Token::new(TokenKind::LBrace, Span::new(10, 1)),
            Token::new(TokenKind::RBrace, Span::new(0, 1)),
        ];

        // `types_section` covers (20, 5) through (0, 1) — a negative width,
        // degraded to zero — and then the stream ends before `graph`.
        let diagnostic = parse(tokens).expect_err("the stream ends before `graph`");

        assert_eq!(diagnostic.class.id(), "unexpected_token");
        assert_eq!(diagnostic.span, Some(Span::new(1, 0)));
    }

    #[test]
    fn no_section_admits_a_body_yet() {
        assert_eq!(
            failure("module App {\n  types { x }\n  graph {}\n  behaviors {}\n}\n"),
            ("unexpected_token", Span::new(23, 1))
        );
        assert_eq!(
            failure("module App {\n  types {}\n  graph { x }\n  behaviors {}\n}\n"),
            ("unexpected_token", Span::new(34, 1))
        );
        assert_eq!(
            failure("module App {\n  types {}\n  graph {}\n  behaviors { x }\n}\n"),
            ("unexpected_token", Span::new(49, 1))
        );
    }
}
