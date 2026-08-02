//! [`parse()`] — the recursive-descent parser: one method per IR_GRAMMAR
//! production (R53).

use std::mem;

use crate::diag::{DiagClass, Diagnostic, Span};
use crate::module::{BehaviorsSection, GraphSection, Module, TypesSection};
use crate::text::keywords::{
    ALIGN, BEHAVIORS, CLOSURE, ENUM, GRAPH, HANDLE, MODULE, POOL_INDEX, PORTAL, RECORD, SIZE,
    TUPLE, TYPES, YIELDED,
};
use crate::text::token::{Token, TokenKind};
use crate::types::{Field, Layout, Prim, TypeDef, TypeTag, Variant};

/// How deep a `type_tag` may nest: 64 bracketed constructors (033-255, R59).
///
/// FORMAT LAW, not implementation latitude — every conforming reader accepts
/// every tag within the bound and rejects every tag beyond it, identically. It
/// is therefore written here as a grammar constant rather than tuned to a stack
/// budget: a reader with more stack must still reject the 65th level.
const TYPE_TAG_DEPTH_LIMIT: usize = 64;

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
/// illegal program. A `types` table may define the same name twice, a field tag
/// may reference a type nobody declares, and a `size` may contradict its
/// layout — all three parse.
///
/// It has two other diagnostic classes, and only two. A STRUCTURAL number that
/// names no `u64` is `int_out_of_range` (R57): not a grammar failure — the
/// token IS an `INT` — but the position's type is known here, so the conversion
/// happens here and so does its failure. A `type_tag` nesting past 64 bracketed
/// constructors is `nesting_too_deep` (033-255, R59): that one IS a grammar
/// failure, since the bound is part of the format rather than a budget of this
/// implementation.
///
/// **The stream is consumed, not borrowed.** R18 gives tokens OWNED payloads so
/// nothing downstream carries a lifetime; taking the `Vec` by value lets those
/// payloads MOVE lexer → parser → AST. A type's name is taken out of its token
/// rather than cloned out of it, and every construct row from here on inherits
/// that shape — the text of a `.duir` file is allocated once.
///
/// **Newlines are optional at module level, and required between entries.**
/// Between the module-level brackets there is no entry to terminate, so a
/// newline is accepted at exactly these positions and required at none: before
/// `module`, after the module's `{`, between sections, inside a section's
/// braces, before the module's `}`, and after it.
///
/// INSIDE a section, 033-250 makes NEWLINE a normative entry terminator: one
/// `type_def` per line, blank lines insignificant, and the section's own `}`
/// terminating a final entry that shares its line. Two entries written on one
/// line are rejected at the second entry's first token.
///
/// Everywhere else a newline is an `unexpected_token` — inside the module
/// header (`module\nApp`), between a section keyword and its brace (`types\n{`),
/// and inside a `type_def`, whose braces are not a braced BODY in 033-250's
/// sense (that paragraph names `scope` and `behavior`). A type row is one line.
///
/// **`graph` and `behaviors` are still empty.** `graph {}` and `behaviors {}`
/// parse; anything else between their braces is an `unexpected_token` until
/// their rows admit entries.
pub fn parse(tokens: Vec<Token>) -> Result<Module, Diagnostic> {
    let mut parser = Parser {
        tokens,
        cursor: 0,
        depth: 0,
    };
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
    /// How many bracketed `type_tag` constructors are open at the cursor
    /// (033-255). Raised and lowered in [`type_tag`](Parser::type_tag) alone.
    depth: usize,
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
    //
    // The `*` is an ENTRY LOOP under 033-250, not a run of tokens to skip: each
    // `type_def` is terminated by a NEWLINE, and the section's `}` terminates a
    // final entry that shares its line. The loop head tests for that `}` and
    // [`entry_terminator`](Parser::entry_terminator) enforces the rest, so two
    // entries on one line fail at the second one's first token.
    fn types_section(&mut self) -> Result<TypesSection, Diagnostic> {
        self.skip_newlines();

        let start = self.expect_word(TYPES)?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        // The line ending after the opening brace terminates no entry, so it is
        // optional the way the ones between entries are not.
        self.skip_newlines();

        let mut type_defs = Vec::new();

        while self.peek_kind() != Some(&TokenKind::RBrace) {
            type_defs.push(self.type_def()?);
            self.entry_terminator()?;
        }

        let end = self.expect(&TokenKind::RBrace, "`}`")?;

        Ok(TypesSection {
            span: covering(start, end),
            type_defs,
        })
    }

    // type_def ::= '%' NAME '=' layout 'size' INT 'align' INT
    //
    // `size` and `align` are STRUCTURAL numbers: converted here, under the
    // position's type, rather than carried to the loader as text (R57).
    fn type_def(&mut self) -> Result<TypeDef, Diagnostic> {
        let start = self.expect(&TokenKind::Percent, "`%`")?;
        let id = self.name("a type name")?;
        self.expect(&TokenKind::Eq, "`=`")?;
        let layout = self.layout()?;

        self.expect_word(SIZE)?;
        let (size, _) = self.structural_int("a size")?;

        self.expect_word(ALIGN)?;
        let (align, end) = self.structural_int("an alignment")?;

        Ok(TypeDef {
            id,
            span: covering(start, end),
            layout,
            size,
            align,
        })
    }

    // layout ::= 'record' '{' field_list '}'
    //          | 'enum'   '{' variant (',' variant)* '}'
    //          | 'tuple'  '(' type_tag (',' type_tag)* ')'
    //
    // The arities differ and the code shows it: a `record`'s field list is
    // optional, so `record {}` is legal; an `enum` and a `tuple` each require at
    // least one element, so neither empty form parses.
    fn layout(&mut self) -> Result<Layout, Diagnostic> {
        if self.at_word(RECORD) {
            self.advance();
            self.expect(&TokenKind::LBrace, "`{`")?;
            let fields = self.field_list()?;
            self.expect(&TokenKind::RBrace, "`}`")?;

            return Ok(Layout::Record(fields));
        }

        if self.at_word(ENUM) {
            self.advance();
            self.expect(&TokenKind::LBrace, "`{`")?;
            let mut variants = vec![self.variant()?];

            while self.peek_kind() == Some(&TokenKind::Comma) {
                self.advance();
                variants.push(self.variant()?);
            }

            self.expect(&TokenKind::RBrace, "`}`")?;

            return Ok(Layout::Enum(variants));
        }

        if self.at_word(TUPLE) {
            self.advance();
            self.expect(&TokenKind::LParen, "`(`")?;
            let elements = self.tag_sequence()?;
            self.expect(&TokenKind::RParen, "`)`")?;

            return Ok(Layout::Tuple(elements));
        }

        Err(self.unexpected(&format!("`{RECORD}`, `{ENUM}` or `{TUPLE}`")))
    }

    // field_list ::= (NAME ':' type_tag (',' NAME ':' type_tag)*)?
    //
    // The whole list is optional, and the only thing that can stand where a
    // first field would is the record's closing `}` — so that is what the empty
    // case tests for.
    fn field_list(&mut self) -> Result<Vec<Field>, Diagnostic> {
        let mut fields = Vec::new();

        if self.peek_kind() == Some(&TokenKind::RBrace) {
            return Ok(fields);
        }

        fields.push(self.field()?);

        while self.peek_kind() == Some(&TokenKind::Comma) {
            self.advance();
            fields.push(self.field()?);
        }

        Ok(fields)
    }

    // variant ::= '#' NAME ('(' type_tag (',' type_tag)* ')')?
    //
    // The payload's optionality is carried by EMPTINESS, not by an `Option`:
    // the grammar admits no `#None()`, so an empty parenthesis pair is not a
    // state a parsed variant can be in.
    fn variant(&mut self) -> Result<Variant, Diagnostic> {
        self.expect(&TokenKind::Hash, "`#`")?;
        let name = self.name("a variant name")?;
        let mut payload = Vec::new();

        if self.peek_kind() == Some(&TokenKind::LParen) {
            self.advance();
            payload = self.tag_sequence()?;
            self.expect(&TokenKind::RParen, "`)`")?;
        }

        Ok(Variant { name, payload })
    }

    // type_tag ::= PRIM
    //            | '%' NAME
    //            | 'pool_index' '<' '%' NAME '>'
    //            | 'yielded' '<' type_tag '>'
    //            | 'handle' '<' type_tag '>'
    //            | 'portal' '<' type_tag '>'
    //            | '(' type_tag (',' type_tag)* ')'
    //            | '[' type_tag ';' INT ']'
    //            | 'closure' '<' '(' (type_tag (',' type_tag)*)? ')' '->' type_tag '>'
    //
    // Ordinary recursive descent: every alternative is distinguished by its
    // FIRST token, so there is no backtracking and no lookahead beyond the
    // cursor. The two vocabularies a type position classifies against — the
    // wrapper keywords and `PRIM` — are disjoint (`text::keywords` pins that),
    // so the order the alternatives are tested in is readability, not
    // correctness.
    //
    // `>` is one token, always (R19): `handle<portal<i32>>` closes with two of
    // them, and each nesting level consumes its own.
    //
    // NESTING (033-255, R59). A tag nests at most `TYPE_TAG_DEPTH_LIMIT`
    // BRACKETED CONSTRUCTORS deep. That is what the law counts, so it is what
    // this counts: the two LEAF alternatives — `PRIM` and `'%' NAME` — open no
    // bracket, cost no level, and are taken first, before the counter is
    // touched. Everything else opens `<…>`, `(…)` or `[…]` and costs one. The
    // check runs BEFORE the constructor's keyword is consumed, so the
    // diagnostic lands on the constructor that exceeds the bound rather than
    // somewhere inside it — and the parser never recurses past the bound, which
    // is what keeps a hostile tag from running the stack out.
    fn type_tag(&mut self) -> Result<TypeTag, Diagnostic> {
        if self.peek_kind() == Some(&TokenKind::Percent) {
            self.advance();

            return Ok(TypeTag::TypeId(self.name("a type name")?));
        }

        if let Some(prim) = self.peek_prim() {
            self.advance();

            return Ok(TypeTag::Prim(prim));
        }

        if self.depth >= TYPE_TAG_DEPTH_LIMIT {
            return Err(self.too_deep());
        }

        self.depth += 1;
        let tag = self.bracketed_tag();
        self.depth -= 1;

        tag
    }

    // The bracketed `type_tag` alternatives — every one that opens `<…>`, `(…)`
    // or `[…]`, and so every one that costs a nesting level. Split out of
    // [`type_tag`](Parser::type_tag) so the level is counted in exactly one
    // place; the leaf alternatives never reach here.
    fn bracketed_tag(&mut self) -> Result<TypeTag, Diagnostic> {
        if self.at_word(POOL_INDEX) {
            self.advance();
            self.expect(&TokenKind::Lt, "`<`")?;
            self.expect(&TokenKind::Percent, "`%`")?;
            let id = self.name("a type name")?;
            self.expect(&TokenKind::Gt, "`>`")?;

            return Ok(TypeTag::PoolIndex(id));
        }

        if self.at_word(YIELDED) {
            self.advance();

            return Ok(TypeTag::Yielded(self.angled_tag()?));
        }

        if self.at_word(HANDLE) {
            self.advance();

            return Ok(TypeTag::Handle(self.angled_tag()?));
        }

        if self.at_word(PORTAL) {
            self.advance();

            return Ok(TypeTag::Portal(self.angled_tag()?));
        }

        if self.at_word(CLOSURE) {
            self.advance();
            self.expect(&TokenKind::Lt, "`<`")?;
            self.expect(&TokenKind::LParen, "`(`")?;
            let mut params = Vec::new();

            if self.peek_kind() != Some(&TokenKind::RParen) {
                params = self.tag_sequence()?;
            }

            self.expect(&TokenKind::RParen, "`)`")?;
            self.expect(&TokenKind::Arrow, "`->`")?;
            let ret = Box::new(self.type_tag()?);
            self.expect(&TokenKind::Gt, "`>`")?;

            return Ok(TypeTag::Closure { params, ret });
        }

        if self.peek_kind() == Some(&TokenKind::LParen) {
            self.advance();
            let elements = self.tag_sequence()?;
            self.expect(&TokenKind::RParen, "`)`")?;

            return Ok(TypeTag::Tuple(elements));
        }

        if self.peek_kind() == Some(&TokenKind::LBracket) {
            self.advance();
            let element = Box::new(self.type_tag()?);
            self.expect(&TokenKind::Semicolon, "`;`")?;
            let (length, _) = self.structural_int("an array length")?;
            self.expect(&TokenKind::RBracket, "`]`")?;

            return Ok(TypeTag::Array(element, length));
        }

        Err(self.unexpected("a type tag"))
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
            &format!("`}}` (the `{GRAPH}` section admits no scopes yet)"),
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
            &format!("`}}` (the `{BEHAVIORS}` section admits no behaviors yet)"),
        )?;

        Ok(BehaviorsSection {
            span: covering(start, end),
        })
    }

    // The entry terminator, normative per 033-250: a NEWLINE ends an entry, and
    // the enclosing section's `}` ends a final entry sharing its line. NOTHING
    // else stands between two entries, which is what will keep a later `cell`
    // line's trailing `gate PATH` clause from swallowing the `gate` entry
    // written under it.
    //
    // Blank lines are insignificant: the lexer already collapses a newline run
    // to one token, and the trailing skip covers a hand-built stream too.
    //
    // Shared on purpose — `graph`'s rows and `behaviors`' rows are terminated by
    // the same law and will call this same helper.
    fn entry_terminator(&mut self) -> Result<(), Diagnostic> {
        if self.peek_kind() == Some(&TokenKind::RBrace) {
            return Ok(());
        }

        self.expect(&TokenKind::Newline, "a line ending or `}` after an entry")?;
        self.skip_newlines();

        Ok(())
    }

    // `NAME ':' type_tag` — one element of a `field_list`. The grammar spells
    // the pair out twice inside that production rather than naming it; the
    // parser names it because writing it twice is what invites the two copies
    // to drift.
    fn field(&mut self) -> Result<Field, Diagnostic> {
        let name = self.name("a field name")?;
        self.expect(&TokenKind::Colon, "`:`")?;
        let tag = self.type_tag()?;

        Ok(Field { name, tag })
    }

    // `type_tag (',' type_tag)*` — the non-empty tag run three productions
    // share: a `tuple` layout's elements, a variant's payload, and the tuple
    // tag's elements. A closure's parameter list is the same run made optional,
    // and calls this after testing for the empty case.
    fn tag_sequence(&mut self) -> Result<Vec<TypeTag>, Diagnostic> {
        let mut tags = vec![self.type_tag()?];

        while self.peek_kind() == Some(&TokenKind::Comma) {
            self.advance();
            tags.push(self.type_tag()?);
        }

        Ok(tags)
    }

    // `'<' type_tag '>'` — the wrapper body `yielded`, `handle` and `portal`
    // share. The keyword is already consumed when this is called.
    fn angled_tag(&mut self) -> Result<Box<TypeTag>, Diagnostic> {
        self.expect(&TokenKind::Lt, "`<`")?;
        let tag = self.type_tag()?;
        self.expect(&TokenKind::Gt, "`>`")?;

        Ok(Box::new(tag))
    }

    // A STRUCTURAL number (R57): an `INT` lexeme converted to `u64` right here,
    // because the POSITION fixes the type — a `size`, an `align`, an array
    // length. A signed or oversized spelling is `int_out_of_range` at the
    // literal's own span; it is not an `unexpected_token`, because the token is
    // exactly the lexeme the grammar asked for and only its VALUE is wrong.
    //
    // TYPED values (`init`, `else`) get the opposite treatment and stay raw
    // text until the loader converts them under the cell's type, which is the
    // only place that type is known.
    //
    // Conversion is by VALUE, not by spelling: `007` and `7` are the same
    // number, and since the emitter writes the `u64` rather than the text it
    // came from, a leading-zero spelling round-trips as the canonical `7`. That
    // is the same normalization the emitter performs everywhere else, not a
    // special case.
    //
    // One spelling `u64::from_str` admits that the `INT` lexeme does not is a
    // leading `+` (033-247 puts only a `-` inside the lexeme). `lex` can never
    // produce one — `+` begins no token of the inventory — so it can only
    // arrive from a hand-built stream, and it is rejected explicitly rather
    // than accepted by the standard library's more generous reading.
    //
    // The span comes back with the value so a caller can end a construct's span
    // at the literal it just read.
    fn structural_int(&mut self, expected: &str) -> Result<(u64, Span), Diagnostic> {
        let span = self.span();

        let text = match self.tokens.get_mut(self.cursor) {
            Some(Token {
                kind: TokenKind::Int(text),
                ..
            }) => mem::take(text),
            _ => return Err(self.unexpected(expected)),
        };

        self.advance();

        let converted = if text.starts_with('+') {
            None
        } else {
            text.parse::<u64>().ok()
        };

        let Some(value) = converted else {
            return Err(Diagnostic {
                class: DiagClass::IntOutOfRange,
                span: Some(span),
                path: None,
                message: format!(
                    "{expected} is a whole number 0..={}, found `{text}`",
                    u64::MAX
                ),
            });
        };

        Ok((value, span))
    }

    // What the token at the cursor IS, or `None` past the end of the stream.
    // `lex` always ends a stream in `Eof`, so `None` is reachable only from a
    // hand-built slice — but a parser that panics on one is worse than a parser
    // that reports `unexpected_token`.
    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.cursor).map(|token| &token.kind)
    }

    // Whether the cursor sits on the word `word` — soft-keyword classification
    // without consuming it (R46). The comparison reads the payload in place and
    // allocates nothing.
    fn at_word(&self, word: &str) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Ident(text)) if text == word)
    }

    // The primitive the word at the cursor names, if it names one. `Prim` is
    // `Copy`, so this hands back a value rather than a borrow and the caller is
    // free to advance.
    fn peek_prim(&self) -> Option<Prim> {
        match self.peek_kind() {
            Some(TokenKind::Ident(text)) => Prim::from_word(text),
            _ => None,
        }
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
    fn expect_word(&mut self, word: &str) -> Result<Span, Diagnostic> {
        if self.at_word(word) {
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
    // This is the MODULE-LEVEL tolerance and nothing else. An entry loop must
    // never open by skipping newlines — that silently accepts two entries
    // written on one line, which 033-250 forbids. Entries are terminated by
    // `entry_terminator`, which requires what this one tolerates. Module level
    // may skip freely for one reason only: between the module-level brackets
    // there is no entry to terminate.
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

    // The nesting bound, reported at the constructor that exceeds it (033-255).
    // Same cursor rule as `unexpected`: the offending token is the one the
    // parser is sitting on, un-consumed.
    fn too_deep(&self) -> Diagnostic {
        Diagnostic {
            class: DiagClass::NestingTooDeep,
            span: Some(self.span()),
            path: None,
            message: format!(
                "a type tag nests at most {TYPE_TAG_DEPTH_LIMIT} levels deep, found {} at level {}",
                describe(self.peek_kind()),
                self.depth + 1
            ),
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
    use crate::text::test_support::{BLANK_LINES, CANONICAL, ENTRY, ONE_LINE, WITH_TYPES};
    use crate::text::test_support::{parsed, with_types};

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
        assert!(module.types.type_defs.is_empty());
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
        for name in [TYPES, GRAPH, BEHAVIORS, MODULE, RECORD, SIZE, POOL_INDEX] {
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
    fn graph_and_behaviors_admit_no_body_yet() {
        assert_eq!(
            failure("module App {\n  types {}\n  graph { x }\n  behaviors {}\n}\n"),
            ("unexpected_token", Span::new(34, 1))
        );
        assert_eq!(
            failure("module App {\n  types {}\n  graph {}\n  behaviors { x }\n}\n"),
            ("unexpected_token", Span::new(49, 1))
        );
    }

    // ---- types table ----

    #[test]
    fn a_type_def_parses_with_hand_counted_spans() {
        assert_eq!(WITH_TYPES.len(), 106);
        // The two anchors every span below is written against: the fixture is
        // what `with_types` builds, and an entry's first byte is `ENTRY`.
        assert_eq!(
            with_types("%Request = record { text: str } size 8 align 8"),
            WITH_TYPES
        );
        assert_eq!(WITH_TYPES.find('%'), Some(ENTRY));

        let module = parsed(WITH_TYPES);

        // `module` through the final `}` at 104.
        assert_eq!(module.span, Span::new(0, 105));
        // `types` at 15 through the section's `}` at 76.
        assert_eq!(module.types.span, Span::new(15, 62));
        assert_eq!(module.graph.span, Span::new(80, 8));
        assert_eq!(module.behaviors.span, Span::new(91, 12));

        let [type_def] = module.types.type_defs.as_slice() else {
            panic!("the section holds exactly one row");
        };

        assert_eq!(type_def.id, "Request");
        // `%` at 27 through the `align` literal at 72.
        assert_eq!(type_def.span, Span::new(ENTRY, 46));
        assert_eq!(type_def.size, 8);
        assert_eq!(type_def.align, 8);

        let Layout::Record(fields) = &type_def.layout else {
            panic!("the layout is a record");
        };

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "text");
        assert!(matches!(fields[0].tag, TypeTag::Prim(Prim::Str)));
    }

    #[test]
    fn each_layout_parses_and_round_trips() {
        for entry in [
            "%R = record { text: str, count: i32 } size 16 align 8",
            "%E = enum { #Some(i32), #None } size 8 align 4",
            "%T = tuple(i32, f64) size 16 align 8",
            "%Empty = record {} size 0 align 1",
        ] {
            let source = with_types(entry);

            assert_eq!(emit(&parsed(&source)), source);
        }
    }

    #[test]
    fn an_empty_record_parses_to_no_fields() {
        let module = parsed(&with_types("%Empty = record {} size 0 align 1"));

        let Layout::Record(fields) = &module.types.type_defs[0].layout else {
            panic!("the layout is a record");
        };

        assert!(fields.is_empty());
        assert_eq!(module.types.type_defs[0].size, 0);
        assert_eq!(module.types.type_defs[0].align, 1);
    }

    // A variant with a payload and one without, in the same enum: the two
    // states `Variant::payload` can be in.
    #[test]
    fn a_variant_carries_its_payload_by_emptiness() {
        let module = parsed(&with_types(
            "%E = enum { #Some(i32, str), #None } size 24 align 8",
        ));

        let Layout::Enum(variants) = &module.types.type_defs[0].layout else {
            panic!("the layout is an enum");
        };

        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].name, "Some");
        assert_eq!(variants[0].payload.len(), 2);
        assert_eq!(variants[1].name, "None");
        assert!(variants[1].payload.is_empty());
    }

    // Every alternative of the `type_tag` production, in one entry, in the
    // production's own order. Round-tripping the entry proves both directions.
    #[test]
    fn every_type_tag_form_parses_and_round_trips() {
        let source = with_types(concat!(
            "%All = record { ",
            "a: i32, ",
            "b: %Request, ",
            "c: pool_index<%Request>, ",
            "d: yielded<i32>, ",
            "e: handle<i32>, ",
            "f: portal<i32>, ",
            "g: (i32, str), ",
            "h: [i32; 4], ",
            "i: closure<(i32) -> str>, ",
            "j: closure<() -> str>",
            " } size 0 align 1"
        ));
        let module = parsed(&source);

        let Layout::Record(fields) = &module.types.type_defs[0].layout else {
            panic!("the layout is a record");
        };

        assert_eq!(fields.len(), 10);
        assert!(matches!(fields[0].tag, TypeTag::Prim(Prim::I32)));
        assert!(matches!(&fields[1].tag, TypeTag::TypeId(id) if id == "Request"));
        assert!(matches!(&fields[2].tag, TypeTag::PoolIndex(id) if id == "Request"));
        assert!(matches!(fields[3].tag, TypeTag::Yielded(_)));
        assert!(matches!(fields[4].tag, TypeTag::Handle(_)));
        assert!(matches!(fields[5].tag, TypeTag::Portal(_)));
        assert!(matches!(&fields[6].tag, TypeTag::Tuple(elements) if elements.len() == 2));
        assert!(matches!(fields[7].tag, TypeTag::Array(_, 4)));
        assert!(matches!(&fields[8].tag, TypeTag::Closure { params, .. } if params.len() == 1));
        assert!(matches!(&fields[9].tag, TypeTag::Closure { params, .. } if params.is_empty()));
        assert_eq!(emit(&module), source);
    }

    // `>` is one token, so every nesting level closes with its own. Nothing
    // munches `>>` into a shift. These are shallow on purpose — the bound
    // itself is the next test's subject.
    #[test]
    fn nested_tags_round_trip() {
        for entry in [
            "%N = record { deep: handle<portal<handle<portal<i32>>>> } size 8 align 8",
            "%N = record { deep: [[[i32; 2]; 3]; 4] } size 8 align 8",
            "%N = record { deep: (i32, (bool, (f32, str))) } size 8 align 8",
            "%N = record { deep: closure<(closure<() -> i32>, [%N; 2]) -> (bool, str)> } size 8 align 8",
            "%N = tuple(pool_index<%N>, yielded<handle<%N>>) size 8 align 8",
        ] {
            let source = with_types(entry);

            assert_eq!(emit(&parsed(&source)), source);
        }
    }

    // 033-255's nesting bound, both sides of it. What the law counts is
    // BRACKETED CONSTRUCTORS, so 64 `portal<` wrappers around a leaf is exactly
    // the bound and the 65th is one too many.
    //
    // 64 levels of recursive descent is a few kilobytes of stack — comfortable
    // on the 2 MiB a test thread gets — and the parser never goes deeper than
    // that, because the check fires before the offending constructor's own
    // recursion begins.
    #[test]
    fn a_type_tag_nests_at_most_sixty_four_levels() {
        let at_bound = with_types(&format!(
            "%N = record {{ deep: {} }} size 8 align 8",
            portals(TYPE_TAG_DEPTH_LIMIT)
        ));

        assert_eq!(emit(&parsed(&at_bound)), at_bound);

        // The tag starts 20 bytes into the entry (`%N = record { deep: `), and
        // each accepted `portal<` is 7 bytes — so the 65th `portal` keyword,
        // 6 bytes long, begins after 64 of them.
        assert_eq!(
            failure(&with_types(&format!(
                "%N = record {{ deep: {} }} size 8 align 8",
                portals(TYPE_TAG_DEPTH_LIMIT + 1)
            ))),
            (
                "nesting_too_deep",
                Span::new(ENTRY + 20 + TYPE_TAG_DEPTH_LIMIT * 7, 6)
            )
        );
    }

    // `portal<…>` wrapped `levels` deep around an `i32` leaf. The leaf opens no
    // bracket, so it costs no level.
    fn portals(levels: usize) -> String {
        format!("{}i32{}", "portal<".repeat(levels), ">".repeat(levels))
    }

    // Every primitive is a type tag, and only those sixteen words are. `i7` is
    // not a narrower integer — it is not a primitive.
    #[test]
    fn every_primitive_word_parses_in_a_type_position() {
        for word in [
            "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "isize", "usize",
            "f32", "f64", "bool", "str",
        ] {
            let source = with_types(&format!("%P = record {{ v: {word} }} size 0 align 1"));

            assert_eq!(emit(&parsed(&source)), source);
        }

        // `i7` at offset 17 of the entry, two bytes long.
        assert_eq!(
            failure(&with_types("%P = record { v: i7 } size 0 align 1")),
            ("unexpected_token", Span::new(ENTRY + 17, 2))
        );
    }

    // 033-250's entry-termination law, both halves. A NEWLINE ends an entry;
    // the section's `}` ends a final entry sharing its line; nothing else
    // stands between two entries.
    #[test]
    fn two_entries_on_one_line_are_rejected_at_the_second_one() {
        // The second `%` is 30 bytes into the entry line.
        assert_eq!(
            failure(&with_types(
                "%A = record {} size 0 align 1 %B = record {} size 0 align 1"
            )),
            ("unexpected_token", Span::new(ENTRY + 30, 1))
        );
    }

    #[test]
    fn a_final_entry_may_share_the_sections_closing_brace() {
        let one_line =
            "module App { types { %A = record {} size 0 align 1 } graph {} behaviors {} }";
        let canonical = with_types("%A = record {} size 0 align 1");

        assert_eq!(emit(&parsed(one_line)), canonical);
        // And the §5 spelling of the same idea, extra spaces included.
        assert_eq!(
            emit(&parsed(
                "module App {\n  types { %Request = record { text: str }  size 8 align 8 }\n  graph {}\n  behaviors {}\n}\n"
            )),
            WITH_TYPES
        );
    }

    #[test]
    fn blank_lines_between_entries_are_insignificant() {
        let spaced = "module App {\n  types {\n\n    %A = record {} size 0 align 1\n\n\n    %B = record {} size 0 align 1\n\n  }\n  graph {}\n  behaviors {}\n}\n";
        let module = parsed(spaced);

        assert_eq!(module.types.type_defs.len(), 2);
        assert_eq!(
            emit(&module),
            "module App {\n  types {\n    %A = record {} size 0 align 1\n    %B = record {} size 0 align 1\n  }\n  graph {}\n  behaviors {}\n}\n"
        );
    }

    // A `type_def` is one line. 033-250's braced-body latitude names `scope`
    // and `behavior`; a record's braces are not a body in that sense, so a
    // newline inside one is an ordinary unexpected token.
    #[test]
    fn a_newline_inside_a_type_def_is_rejected() {
        // The newline closing `record {` sits at byte 40.
        assert_eq!(
            failure(
                "module App {\n  types {\n    %A = record {\n      x: i32\n    } size 0 align 1\n  }\n  graph {}\n  behaviors {}\n}\n"
            ),
            ("unexpected_token", Span::new(40, 1))
        );
    }

    // R57: a structural number is converted here, and a spelling that names no
    // `u64` fails at the literal — not as an `unexpected_token`, because the
    // token is exactly the `INT` the grammar asked for.
    #[test]
    fn a_structural_number_must_name_a_u64() {
        // The `size` literal starts 20 bytes into the entry; the overflowing
        // one is 20 digits long, the signed one 2 bytes (the `-` is inside the
        // lexeme, 033-247).
        assert_eq!(
            failure(&with_types(
                "%A = record {} size 99999999999999999999 align 8"
            )),
            ("int_out_of_range", Span::new(ENTRY + 20, 20))
        );
        assert_eq!(
            failure(&with_types("%A = record {} size -8 align 8")),
            ("int_out_of_range", Span::new(ENTRY + 20, 2))
        );
        // `align` is converted by the same helper and fails the same way — the
        // literal starts 28 bytes into the entry.
        assert_eq!(
            failure(&with_types(
                "%A = record {} size 8 align 99999999999999999999"
            )),
            ("int_out_of_range", Span::new(ENTRY + 28, 20))
        );
        // An array length is structural too — `[i32; -1]` puts the literal 23
        // bytes in, 2 bytes long.
        assert_eq!(
            failure(&with_types("%A = record { v: [i32; -1] } size 0 align 1")),
            ("int_out_of_range", Span::new(ENTRY + 23, 2))
        );
        // The boundary itself parses.
        let module = parsed(&with_types(
            "%A = record {} size 18446744073709551615 align 1",
        ));

        assert_eq!(module.types.type_defs[0].size, u64::MAX);
    }

    // Leading zeros are a SPELLING, not a value. The parser converts, the
    // emitter writes the number back, and the canonical digits are what comes
    // out — the same normalization non-canonical spacing gets.
    #[test]
    fn a_leading_zero_spelling_converts_and_emits_canonically() {
        let module = parsed(&with_types(
            "%A = record { v: [i32; 007] } size 0064 align 08",
        ));

        assert_eq!(module.types.type_defs[0].size, 64);
        assert_eq!(module.types.type_defs[0].align, 8);

        let Layout::Record(fields) = &module.types.type_defs[0].layout else {
            panic!("the layout is a record");
        };

        let TypeTag::Array(_, length) = &fields[0].tag else {
            panic!("the field's tag is an array");
        };

        assert_eq!(*length, 7);
        assert_eq!(
            emit(&module),
            with_types("%A = record { v: [i32; 7] } size 64 align 8")
        );
    }

    // `u64::from_str` admits a leading `+`; the `INT` lexeme does not (033-247
    // puts only a `-` inside the lexeme). No lexer output can carry one, since
    // `+` begins no token of the inventory — so the stream is patched by hand,
    // which is exactly the case the explicit guard exists for.
    #[test]
    fn a_plus_signed_structural_number_is_out_of_range() {
        let mut tokens = lex(&with_types("%A = record {} size 7 align 1")).expect("source lexes");

        for token in &mut tokens {
            if token.kind == TokenKind::Int("7".to_string()) {
                token.kind = TokenKind::Int("+7".to_string());
            }
        }

        let diagnostic = parse(tokens).expect_err("a `+`-signed literal fails to parse");

        assert_eq!(diagnostic.class.id(), "int_out_of_range");
        // The `size` literal, at its original span.
        assert_eq!(diagnostic.span, Some(Span::new(ENTRY + 20, 1)));
    }

    #[test]
    fn a_malformed_type_def_reports_at_the_offending_token() {
        // Missing `=`: `record` stands where it belongs, 9 bytes in.
        assert_eq!(
            failure(&with_types("%Request record { text: str } size 8 align 8")),
            ("unexpected_token", Span::new(ENTRY + 9, 6))
        );
        // Missing `size`: `align` turns up in its place, 32 bytes in.
        assert_eq!(
            failure(&with_types("%Request = record { text: str } align 8")),
            ("unexpected_token", Span::new(ENTRY + 32, 5))
        );
        // A layout keyword that is not one of the three, 11 bytes in.
        assert_eq!(
            failure(&with_types(
                "%Request = struct { text: str } size 8 align 8"
            )),
            ("unexpected_token", Span::new(ENTRY + 11, 6))
        );
        // An unclosed `<`: the record's `}` arrives where `>` belongs, 46 bytes
        // in.
        assert_eq!(
            failure(&with_types(
                "%Request = record { body: pool_index<%Payload } size 8 align 8"
            )),
            ("unexpected_token", Span::new(ENTRY + 46, 1))
        );
        // An enum needs at least one variant: its `}` is 17 bytes in.
        assert_eq!(
            failure(&with_types("%Request = enum {} size 8 align 8")),
            ("unexpected_token", Span::new(ENTRY + 17, 1))
        );
        // And a variant needs a payload inside parentheses if it opens them:
        // the `)` is 24 bytes in.
        assert_eq!(
            failure(&with_types("%Request = enum { #None() } size 8 align 8")),
            ("unexpected_token", Span::new(ENTRY + 24, 1))
        );
    }
}
