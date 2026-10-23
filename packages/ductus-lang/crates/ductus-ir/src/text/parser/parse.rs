//! [`parse()`] — the recursive-descent parser: one method per IR_GRAMMAR
//! production (R53).

use std::mem;

use crate::diag::{DiagClass, Diagnostic, Span};
use crate::graph::{
    Binding, BindingTarget, Cell, CellKind, Effect, Entry, FallbackEntry, Gate, LookbackEntry,
    Member, MemberDriver, Provenance, ReopenPair, Scope, Stream, StreamPolicy, ValueLiteral, Wire,
    WireTo,
};
use crate::module::{BehaviorsSection, GraphSection, Module, TypesSection};
use crate::path::{CellPath, Segment};
use crate::text::is_reserved_word;
use crate::text::keywords::{
    ALIGN, ATTRS, BEHAVIORS, CAPACITY, CLOSURE, COMBINER, DEPTH, DESIRED, EFFECT, EFFECTS, ELSE,
    ENUM, EXPOSES, FALLBACKS, FALSE, FROM, GATE, GATE_PARENT, GRAPH, GUARDS, HANDLE, HISTORY, INF,
    INIT, INPUTS, LOOKBACK, MEMBERS, MODULE, NAN, NULL, OBSERVED, OBSERVES, PARAMS, POLICY,
    POOL_INDEX, PORTAL, PRED, RECONCILER, RECORD, RESET_ON_RELOAD, RESET_ON_REOPEN, SCOPE, SIZE,
    SOURCE_DEPS, STREAM, TO, TRUE, TUPLE, TYPE, TYPES, USES, WIRE, YIELDED,
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

/// What a `':' INT` inside a PATH calls itself when its value names no `u64`
/// (R57's expected-text).
const PATH_ORDINAL: &str = "a path ordinal";

/// The same, inside a `lookback_map` entry — where a trailing `':' INT` is the
/// lookback VALUE (R43) and not an ordinal at all.
const LOOKBACK_VALUE: &str = "a lookback value";

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
/// It has three other diagnostic classes, and only three. A STRUCTURAL number
/// that names no `u64` is `int_out_of_range` (R57): not a grammar failure — the
/// token IS an `INT` — but the position's type is known here, so the conversion
/// happens here and so does its failure. A `type_tag` nesting past 64 bracketed
/// constructors is `nesting_too_deep` (033-255, R59): that one IS a grammar
/// failure, since the bound is part of the format rather than a budget of this
/// implementation. And one of the five RESERVED spellings — `null`, `true`,
/// `false`, `nan`, `inf` — standing where a `NAME` is required is
/// `reserved_word` (002-39, 033-259): also a grammar failure, split from
/// `unexpected_token` because the token IS word-shaped and the position DOES
/// want a word, so the useful message is about the spelling.
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
/// A `scope` IS such a braced body: it may span physical lines between its
/// braces, its own entries are terminated by NEWLINE or by its closing `}`, and
/// the scope entry itself is terminated by the NEWLINE after that `}`. The same
/// `entry_terminator` enforces both levels.
///
/// **`behaviors` is still empty.** `behaviors {}` parses; anything else between
/// its braces is an `unexpected_token` until its row admits entries. The
/// `graph` section admits `scope*` (033-256, FND-16 (b)), so an empty
/// `graph {}` is legal rather than tolerated.
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

    // graph_section ::= 'graph' '{' scope* '}'
    //
    // The arity is `scope*` (033-256): an empty graph is a module of pure
    // types, legal by the grammar, and any minimum-content requirement is the
    // loader's. The `*` is the same ENTRY LOOP the `types` table runs — a scope
    // is an entry of this section, terminated by the NEWLINE after its closing
    // brace (033-250) — so the loop and its terminator are shared verbatim.
    fn graph_section(&mut self) -> Result<GraphSection, Diagnostic> {
        self.skip_newlines();

        let start = self.expect_word(GRAPH)?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        self.skip_newlines();

        let mut scopes = Vec::new();

        while self.peek_kind() != Some(&TokenKind::RBrace) {
            scopes.push(self.scope()?);
            self.entry_terminator()?;
        }

        let end = self.expect(&TokenKind::RBrace, "`}`")?;

        Ok(GraphSection {
            span: covering(start, end),
            scopes,
        })
    }

    // scope ::= 'scope' PATH 'exposes' path_set 'effects' path_set
    //           ('reset_on_reopen' reopen_set)? '{' entry* '}'
    //
    // Scopes are FLAT under the section: a scope holds entries, never other
    // scopes, and the nesting a program has rides each scope's PATH instead
    // (§15.4.1.1). So `scope App.panel` is a sibling of `scope App` here and a
    // child of it by name.
    //
    // `exposes` and `effects` are MANDATORY, `reset_on_reopen` optional — the
    // first of that keyword's three grammar positions, the one that carries a
    // payload (033-215).
    fn scope(&mut self) -> Result<Scope, Diagnostic> {
        let start = self.expect_word(SCOPE)?;
        let path = self.path()?;

        self.expect_word(EXPOSES)?;
        let exposes = self.path_set()?;

        self.expect_word(EFFECTS)?;
        let effects = self.path_set()?;

        let mut reset_on_reopen = Vec::new();

        if self.at_word(RESET_ON_REOPEN) {
            self.advance();
            reset_on_reopen = self.reopen_set()?;
        }

        self.expect(&TokenKind::LBrace, "`{`")?;
        // The line ending after the opening brace terminates no entry, so it is
        // optional the way the ones between entries are not.
        self.skip_newlines();

        let mut entries = Vec::new();

        while self.peek_kind() != Some(&TokenKind::RBrace) {
            entries.push(self.entry()?);
            self.entry_terminator()?;
        }

        let end = self.expect(&TokenKind::RBrace, "`}`")?;

        Ok(Scope {
            path,
            span: covering(start, end),
            exposes,
            effects,
            reset_on_reopen,
            entries,
        })
    }

    // entry ::= cell | gate | wire | effect | stream
    //
    // Every alternative is decided by its leading WORD — one of the five cell
    // kinds, or one of the four entry heads — so there is no lookahead and no
    // backtracking here either.
    fn entry(&mut self) -> Result<Entry, Diagnostic> {
        if self.peek_cell_kind().is_some() {
            return Ok(Entry::Cell(self.cell()?));
        }

        if self.at_word(GATE) {
            return Ok(Entry::Gate(self.gate()?));
        }

        if self.at_word(WIRE) {
            return Ok(Entry::Wire(self.wire()?));
        }

        if self.at_word(EFFECT) {
            return Ok(Entry::Effect(self.effect()?));
        }

        if self.at_word(STREAM) {
            return Ok(Entry::Stream(self.stream()?));
        }

        Err(self.unexpected(&format!(
            "a cell kind, `{GATE}`, `{WIRE}`, `{EFFECT}` or `{STREAM}`"
        )))
    }

    // cell ::= cell_kind PATH ':' type_tag
    //          ('uses' BID)? ('inputs' path_set)? ('depth' INT)?
    //          ('lookback' lookback_map)?
    //          ('reset_on_reopen')? ('init' value)?
    //          ('fallbacks' fallback_map)? ('gate' PATH)?
    //          ('combiner' BID)? ('else' value)? ('members' member_set)?
    //
    // Eleven optional clauses, each admitted AT MOST ONCE and only in the
    // production's order — which is what reading them SEQUENTIALLY means: a
    // clause written out of order, or twice, does not match at its turn, so the
    // entry ends before it and the terminator rejects it as a second entry on
    // one line. That is the diagnostic a clause-order violation gets, and it
    // points at the offending clause's own word.
    //
    // Which clauses a KIND admits is not checked here (F4, R21): `input App.x :
    // i32 combiner B@1` parses, and the loader rejects it under the kind's
    // rules. So does `init` TOGETHER WITH `fallbacks` on a recurrent — one
    // clause in two spellings, and carrying both is a load error (033-268), not
    // a grammar failure. `depth` and every `lookback` / `fallbacks` KEY are
    // structural and convert now (R57); `init`, `else` and every `fallbacks`
    // VALUE are typed and stay text.
    fn cell(&mut self) -> Result<Cell, Diagnostic> {
        let (kind, start) = self.cell_kind()?;
        let path = self.path()?;

        self.expect(&TokenKind::Colon, "`:`")?;
        let tag = self.type_tag()?;

        let uses = self.optional_bid(USES)?;
        let inputs = self.optional_path_set(INPUTS)?;
        let depth = self.optional_structural_int(DEPTH, "a depth")?;
        let lookback = self.optional_list(LOOKBACK, Parser::lookback_map)?;
        let reset_on_reopen = self.flag(RESET_ON_REOPEN);
        let init = self.optional_value(INIT)?;
        let fallbacks = self.optional_list(FALLBACKS, Parser::fallback_map)?;
        let gate = self.optional_path(GATE)?;
        let combiner = self.optional_bid(COMBINER)?;
        let else_value = self.optional_value(ELSE)?;
        let members = self.optional_list(MEMBERS, Parser::member_set)?;

        Ok(Cell {
            kind,
            path,
            span: covering(start, self.previous_span()),
            tag,
            uses,
            inputs,
            depth,
            lookback,
            reset_on_reopen,
            init,
            fallbacks,
            gate,
            combiner,
            else_value,
            members,
        })
    }

    // cell_kind ::= 'input' | 'derived' | 'recurrent' | 'deferred' | 'fold'
    //
    // The five spellings live on `CellKind` itself, the way the sixteen
    // primitive spellings live on `Prim`: a closed set is a type, not a list of
    // words in the keyword table.
    fn cell_kind(&mut self) -> Result<(CellKind, Span), Diagnostic> {
        match self.peek_cell_kind() {
            Some(kind) => Ok((kind, self.advance())),
            None => Err(self.unexpected("a cell kind")),
        }
    }

    // member_set ::= '[' (member (',' member)*)? ']'
    fn member_set(&mut self) -> Result<Vec<Member>, Diagnostic> {
        self.bracketed(Parser::member)
    }

    // member ::= PATH ':' member_driver
    //
    // The pair separator is `:` and a path segment may end with a `:N` ordinal.
    // Nothing here disambiguates them, because [`path`](Parser::path) already
    // does: after a colon an INT continues the path and a word does not, which
    // is §3's boundary note applied where paths are read.
    fn member(&mut self) -> Result<Member, Diagnostic> {
        let path = self.path()?;

        self.expect(&TokenKind::Colon, "`:`")?;
        let driver = self.member_driver()?;

        Ok(Member { path, driver })
    }

    // member_driver ::= 'permanent' | 'keyed_template' | 'gate_guarded'
    fn member_driver(&mut self) -> Result<MemberDriver, Diagnostic> {
        let driver = match self.peek_kind() {
            Some(TokenKind::Ident(text)) => MemberDriver::from_word(text),
            _ => None,
        };

        match driver {
            Some(driver) => {
                self.advance();

                Ok(driver)
            }
            None => Err(self.unexpected("a member driver")),
        }
    }

    // gate ::= 'gate' PATH 'pred' BID 'inputs' path_set 'guards' path_set
    //          ('gate_parent' PATH)?
    //
    // `inputs` and `guards` are MANDATORY here, unlike the `inputs` on a cell:
    // an empty vector means the text spelled `[]`.
    fn gate(&mut self) -> Result<Gate, Diagnostic> {
        let start = self.expect_word(GATE)?;
        let path = self.path()?;

        self.expect_word(PRED)?;
        let pred = self.bid("a behavior handle")?;

        self.expect_word(INPUTS)?;
        let inputs = self.path_set()?;

        self.expect_word(GUARDS)?;
        let guards = self.path_set()?;

        let gate_parent = self.optional_path(GATE_PARENT)?;

        Ok(Gate {
            path,
            span: covering(start, self.previous_span()),
            pred,
            inputs,
            guards,
            gate_parent,
        })
    }

    // wire ::= 'wire' PATH 'from' PATH 'to' (PATH | 'null')
    //          ('type' type_tag)? ('attrs' binding_set)? ('gate' PATH)?
    fn wire(&mut self) -> Result<Wire, Diagnostic> {
        let start = self.expect_word(WIRE)?;
        let path = self.path()?;

        self.expect_word(FROM)?;
        let from = self.path()?;

        self.expect_word(TO)?;
        let to = self.wire_to()?;

        let mut tag = None;

        if self.at_word(TYPE) {
            self.advance();
            tag = Some(self.type_tag()?);
        }

        let mut attrs = Vec::new();

        if self.at_word(ATTRS) {
            self.advance();
            attrs = self.binding_set()?;
        }

        let gate = self.optional_path(GATE)?;

        Ok(Wire {
            path,
            span: covering(start, self.previous_span()),
            from,
            to,
            tag,
            attrs,
            gate,
        })
    }

    // `(PATH | 'null')` — a wire's destination.
    //
    // `null` is RESERVED (002-39), so no PATH can begin with it and the word is
    // the literal outright — no lookahead, nothing shadowed (033-258). `null.x`
    // is not a path either: the word is taken as the whole destination and the
    // `.` after it is the next token, unexpected where the entry has ended. The
    // `reserved_word` class belongs to the NAME positions; here the word is not
    // in one. A key spelled `null` remains reachable, because a KeySegment is
    // quoted and never comes through [`name`](Parser::name).
    fn wire_to(&mut self) -> Result<WireTo, Diagnostic> {
        if self.at_word(NULL) {
            self.advance();

            return Ok(WireTo::Null);
        }

        Ok(WireTo::Path(self.path()?))
    }

    // effect ::= 'effect' PATH 'reconciler' STRING 'params' binding_set
    //            ('desired' path_set)? ('observed' path_set)? ('gate' PATH)?
    fn effect(&mut self) -> Result<Effect, Diagnostic> {
        let start = self.expect_word(EFFECT)?;
        let path = self.path()?;

        self.expect_word(RECONCILER)?;
        let reconciler = self.string("a reconciler name")?;

        self.expect_word(PARAMS)?;
        let params = self.binding_set()?;

        let desired = self.optional_path_set(DESIRED)?;
        let observed = self.optional_path_set(OBSERVED)?;
        let gate = self.optional_path(GATE)?;

        Ok(Effect {
            path,
            span: covering(start, self.previous_span()),
            reconciler,
            params,
            desired,
            observed,
            gate,
        })
    }

    // stream ::= 'stream' PATH ':' type_tag 'policy' ('ring' | 'gate') 'capacity' INT
    //            ('source_deps' path_set)? ('observes' path_set)?
    //            ('reset_on_reload')? ('reset_on_reopen')?
    //            ('history' INT)? ('lookback' lookback_map)?
    //
    // `capacity`, `history` and every lookback `k` are STRUCTURAL (R57). The
    // two bare flags are the third grammar position of `reset_on_reopen`
    // (033-215).
    fn stream(&mut self) -> Result<Stream, Diagnostic> {
        let start = self.expect_word(STREAM)?;
        let path = self.path()?;

        self.expect(&TokenKind::Colon, "`:`")?;
        let tag = self.type_tag()?;

        self.expect_word(POLICY)?;
        let policy = self.stream_policy()?;

        self.expect_word(CAPACITY)?;
        let (capacity, _) = self.structural_int("a capacity")?;

        let source_deps = self.optional_path_set(SOURCE_DEPS)?;
        let observes = self.optional_path_set(OBSERVES)?;
        let reset_on_reload = self.flag(RESET_ON_RELOAD);
        let reset_on_reopen = self.flag(RESET_ON_REOPEN);
        let history = self.optional_structural_int(HISTORY, "a history size")?;
        let lookback = self.optional_list(LOOKBACK, Parser::lookback_map)?;

        Ok(Stream {
            path,
            span: covering(start, self.previous_span()),
            tag,
            policy,
            capacity,
            source_deps,
            observes,
            reset_on_reload,
            reset_on_reopen,
            history,
            lookback,
        })
    }

    // `('ring' | 'gate')` — a stream's policy, spelled inline in the production
    // and owned as a closed set by `StreamPolicy`.
    fn stream_policy(&mut self) -> Result<StreamPolicy, Diagnostic> {
        let policy = match self.peek_kind() {
            Some(TokenKind::Ident(text)) => StreamPolicy::from_word(text),
            _ => None,
        };

        match policy {
            Some(policy) => {
                self.advance();

                Ok(policy)
            }
            None => Err(self.unexpected("a stream policy")),
        }
    }

    // path_set ::= '[' (PATH (',' PATH)*)? ']'
    fn path_set(&mut self) -> Result<Vec<CellPath>, Diagnostic> {
        self.bracketed(Parser::path)
    }

    // binding_set ::= '[' (binding (',' binding)*)? ']'
    fn binding_set(&mut self) -> Result<Vec<Binding>, Diagnostic> {
        self.bracketed(Parser::binding)
    }

    // binding ::= NAME ':' (PATH | value) ('@' provenance)?
    //
    // The provenance clause is optional METADATA on the binding, never a third
    // target case (033-252), so it is read after the target rather than inside
    // it.
    fn binding(&mut self) -> Result<Binding, Diagnostic> {
        let name = self.name("a binding name")?;

        self.expect(&TokenKind::Colon, "`:`")?;
        let target = self.binding_target()?;
        let mut provenance = None;

        if self.peek_kind() == Some(&TokenKind::At) {
            self.advance();
            provenance = Some(self.provenance()?);
        }

        Ok(Binding {
            name,
            target,
            provenance,
        })
    }

    // `(PATH | value)` — the two alternatives a binding binds to.
    //
    // A number, a string and the four value WORDS are values; everything else
    // word-shaped, and every quoted key, begins a path. There is no overlap to
    // resolve: `true`, `false`, `nan` and `inf` are RESERVED (002-39), so no
    // path can be named by one and the word is the value outright, exactly as
    // `null` is on a wire (033-258).
    fn binding_target(&mut self) -> Result<BindingTarget, Diagnostic> {
        if self.at_value_start() {
            return Ok(BindingTarget::Value(self.value()?));
        }

        Ok(BindingTarget::Path(self.path()?))
    }

    // provenance ::= 'bound' | 'constant_wrap' | 'bridge'
    fn provenance(&mut self) -> Result<Provenance, Diagnostic> {
        let provenance = match self.peek_kind() {
            Some(TokenKind::Ident(text)) => Provenance::from_word(text),
            _ => None,
        };

        match provenance {
            Some(provenance) => {
                self.advance();

                Ok(provenance)
            }
            None => Err(self.unexpected("a provenance marker")),
        }
    }

    // reopen_set ::= '[' (PATH ':' PATH (',' PATH ':' PATH)*)? ']'
    fn reopen_set(&mut self) -> Result<Vec<ReopenPair>, Diagnostic> {
        self.bracketed(Parser::reopen_pair)
    }

    // `PATH ':' PATH` — one `(consumer : stream)` pair of a `reopen_set`.
    //
    // §3 states the disambiguator here: after a colon, an INT is the LEFT
    // path's ordinal and a NAME begins the RIGHT path. [`path`](Parser::path)
    // applies exactly that test, so the pair reads as two paths with a colon
    // between them and this needs no lookahead of its own.
    fn reopen_pair(&mut self) -> Result<ReopenPair, Diagnostic> {
        let consumer = self.path()?;

        self.expect(&TokenKind::Colon, "`:`")?;
        let stream = self.path()?;

        Ok(ReopenPair { consumer, stream })
    }

    // lookback_map ::= '[' (PATH ':' INT (',' PATH ':' INT)*)? ']'
    fn lookback_map(&mut self) -> Result<Vec<LookbackEntry>, Diagnostic> {
        self.bracketed(Parser::lookback_entry)
    }

    // `PATH ':' INT` — one `(input cell : k)` pair, under lookback_map's OWN
    // colon rule (R43, §3's scope note).
    //
    // reopen_set's "an INT after a colon is an ordinal" cannot decide here:
    // BOTH sides of this colon-INT are integers. The law is positional instead
    // — the LAST `':' INT` of the entry is the lookback value, and any earlier
    // one belongs to the key path. Reading the key first and then looking at
    // what is left implements exactly that:
    //
    //   `App.print:0 : 3`  a colon still stands after the key, so `3` is the
    //                      value and `:0` stays the ordinal it was read as;
    //   `App.a : 3`        no colon is left, because `:3` was read as `a`'s
    //                      ordinal — it is the entry's last, so it is taken
    //                      back out of the key and becomes the value.
    //
    // The taken-back ordinal must sit on the LAST segment. `App.print:0.text`
    // is not an entry whose value hides in the middle of its key: its last
    // `':' INT` is inside the path, so the entry simply has no value, and the
    // `:` it is missing is what the diagnostic asks for.
    //
    // The key is read under the LOOKBACK VALUE's expected-text rather than an
    // ordinal's, because in this one production a trailing `':' INT` is the
    // value: `App.a: 99999999999999999999` overflows at a literal that is the
    // entry's value, and a diagnostic calling it "a path ordinal" would name
    // the mechanism instead of the position. The text covers every ordinal of
    // the key, so an earlier one — `App.print:<overflowing>: 3` — reports under
    // it too; that spelling is the rare one, and the class and the span are
    // right in both.
    fn lookback_entry(&mut self) -> Result<LookbackEntry, Diagnostic> {
        let cell = self.path_naming_ordinals(LOOKBACK_VALUE)?;

        if self.peek_kind() == Some(&TokenKind::Colon) {
            self.advance();
            let (k, _) = self.structural_int(LOOKBACK_VALUE)?;

            return Ok(LookbackEntry { cell, k });
        }

        // The path is CONSUMED rather than cloned: one segment's ordinal moves
        // out, and nothing else about the path changes.
        let mut segments = cell.into_segments();

        let Some(Segment::Name { ordinal, .. }) = segments.last_mut() else {
            return Err(self.unexpected("`:`"));
        };

        let Some(k) = ordinal.take() else {
            return Err(self.unexpected("`:`"));
        };

        Ok(LookbackEntry {
            cell: CellPath::new(segments),
            k,
        })
    }

    // fallback_map ::= '[' (INT ':' value (',' INT ':' value)*)? ']'
    fn fallback_map(&mut self) -> Result<Vec<FallbackEntry>, Diagnostic> {
        self.bracketed(Parser::fallback_entry)
    }

    // `INT ':' value` — one `(depth : fallback)` pair of a `fallback_map`.
    //
    // NO colon rule is needed here, and that is the point of the key being an
    // INT: an entry has exactly one colon, the key cannot be a PATH, and
    // neither reopen_set's lookahead disambiguator nor lookback_map's
    // positional one (R43) has anything to decide. Key first, colon, value.
    //
    // The two halves are converted the two OPPOSITE ways, as they are
    // everywhere else: `k` is STRUCTURAL and becomes a `u64` now (R57), while
    // the value is TYPED and stays text until the loader reads it under the
    // cell's tag.
    fn fallback_entry(&mut self) -> Result<FallbackEntry, Diagnostic> {
        let (k, _) = self.structural_int("a fallback depth")?;

        self.expect(&TokenKind::Colon, "`:`")?;
        let value = self.value()?;

        Ok(FallbackEntry { k, value })
    }

    // value ::= INT | FLOAT | 'true' | 'false' | STRING | 'nan' | 'inf' | '-inf'
    //
    // The three float spellings are TYPE-GATED (033-248): admitted here
    // wherever a value stands, and rejected outside a float-typed position by
    // the loader's type check rather than by the grammar. `-inf` is not
    // word-shaped, so it arrives as a FLOAT lexeme and `ValueLiteral::float`
    // sorts it from the digits; bare `nan` and `inf` are words.
    //
    // The digits are CANONICALIZED and never converted (033-257, R57): the
    // loader owns the position's type, so the reader normalizes the spelling —
    // `007` to `7`, `1.50` to `1.5` — which is value-safe under every type the
    // position may take.
    fn value(&mut self) -> Result<ValueLiteral, Diagnostic> {
        if let Some(literal) = self.peek_value_word() {
            self.advance();

            return Ok(literal);
        }

        let literal = match self.tokens.get_mut(self.cursor) {
            Some(Token {
                kind: TokenKind::Int(text),
                ..
            }) => ValueLiteral::int(mem::take(text)),
            Some(Token {
                kind: TokenKind::Float(text),
                ..
            }) => ValueLiteral::float(mem::take(text)),
            Some(Token {
                kind: TokenKind::Str(text),
                ..
            }) => ValueLiteral::Str(mem::take(text)),
            _ => return Err(self.unexpected("a value")),
        };

        self.advance();

        Ok(literal)
    }

    // PATH ::= PathSegment ('.' PathSegment)*
    //
    // The lexer composes nothing dotted (033-253): it hands over `Ident`, `Dot`,
    // `Int` and `QuotedKey` tokens, and the path is assembled here.
    fn path(&mut self) -> Result<CellPath, Diagnostic> {
        self.path_naming_ordinals(PATH_ORDINAL)
    }

    // The `PATH` production, with the expected-text an OUT-OF-RANGE ordinal is
    // reported under. Every position but one takes the default, because
    // everywhere else a `':' INT` inside a path IS an ordinal;
    // [`lookback_entry`](Parser::lookback_entry) is the exception and says why.
    fn path_naming_ordinals(&mut self, ordinal: &str) -> Result<CellPath, Diagnostic> {
        let mut segments = vec![self.path_segment(ordinal)?];

        while self.peek_kind() == Some(&TokenKind::Dot) {
            self.advance();
            segments.push(self.path_segment(ordinal)?);
        }

        Ok(CellPath::new(segments))
    }

    // PathSegment ::= NAME (':' INT)? | KeySegment
    //
    // A `BareKey` is the NAME charset exactly (R41), so it lexes as a NAME and
    // is indistinguishable from a name segment in isolation — which costs
    // nothing, because the graph structure decides which positions are keyed
    // (033-243). Only a `QuotedKey` token becomes a key segment here.
    //
    // THE COLON RULE. A colon opens an ordinal ONLY when an INT follows it.
    // That single test settles every colon-adjacent position in the grammar: a
    // `cell`'s and a `stream`'s `PATH ':' type_tag` (a tag never begins with an
    // INT), a `member`'s `PATH ':' member_driver` (a driver is a word), and a
    // `reopen_set` pair's `PATH ':' PATH` (a path begins with a word or a
    // quoted key) — which is precisely the disambiguator §3 states for
    // reopen_set, written once. `lookback_map` is the one position it cannot
    // decide, and it has its own rule and its own method.
    fn path_segment(&mut self, ordinal_expected: &str) -> Result<Segment, Diagnostic> {
        if matches!(self.peek_kind(), Some(TokenKind::QuotedKey(_))) {
            return Ok(Segment::Key(self.quoted_key("a path key")?));
        }

        let text = self.name("a path segment")?;
        let mut ordinal = None;

        if self.peek_kind() == Some(&TokenKind::Colon) && self.peek_int_follows() {
            self.advance();
            let (value, _) = self.structural_int(ordinal_expected)?;
            ordinal = Some(value);
        }

        Ok(Segment::Name { text, ordinal })
    }

    // behaviors_section ::= 'behaviors' '{' behavior* '}'
    //
    // The description is built INSIDE the error path, the way every other
    // interpolated `expected` here is: a section that closes — which is every
    // one the emitter writes — must not pay for a message nobody reads.
    fn behaviors_section(&mut self) -> Result<BehaviorsSection, Diagnostic> {
        self.skip_newlines();

        let start = self.expect_word(BEHAVIORS)?;
        self.expect(&TokenKind::LBrace, "`{`")?;
        self.skip_newlines();

        if self.peek_kind() != Some(&TokenKind::RBrace) {
            return Err(self.unexpected(&format!(
                "`}}` (the `{BEHAVIORS}` section admits no behaviors yet)"
            )));
        }

        let end = self.advance();

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

    // `'[' (element (',' element)*)? ']'` — the bracketed comma list that six
    // productions share: `path_set`, `binding_set`, `member_set`, `reopen_set`,
    // `lookback_map` and `fallback_map`. All six are optional-content lists
    // whose only difference is the element reader, so the brackets, the
    // separator and the empty case are written once.
    fn bracketed<T>(
        &mut self,
        element: impl Fn(&mut Parser) -> Result<T, Diagnostic>,
    ) -> Result<Vec<T>, Diagnostic> {
        self.expect(&TokenKind::LBracket, "`[`")?;
        let mut elements = Vec::new();

        if self.peek_kind() != Some(&TokenKind::RBracket) {
            elements.push(element(self)?);

            while self.peek_kind() == Some(&TokenKind::Comma) {
                self.advance();
                elements.push(element(self)?);
            }
        }

        self.expect(&TokenKind::RBracket, "`]`")?;

        Ok(elements)
    }

    // `word PATH`, or nothing — the optional single-path clause shape shared by
    // `gate` (on a cell, a wire and an effect) and `gate_parent`.
    fn optional_path(&mut self, word: &str) -> Result<Option<CellPath>, Diagnostic> {
        if !self.at_word(word) {
            return Ok(None);
        }

        self.advance();

        Ok(Some(self.path()?))
    }

    // `word <list production>`, or nothing — the shape EVERY optional list
    // clause has: `inputs`, `desired`, `observed`, `source_deps`, `observes`,
    // `members`, `lookback` (on a cell and on a stream) and `fallbacks`.
    //
    // An absent clause and a written `[]` are ONE state (see the entry types),
    // so both come back as an empty vector and the canonical rendering of that
    // state omits the clause. Written once here so no clause can decide that
    // differently.
    //
    // `production` is the LIST production's own method, not its element's — the
    // emitter's `optional_set` takes the same half — so a call site names the
    // production it is reading and every production keeps its own method (R53).
    fn optional_list<T>(
        &mut self,
        word: &str,
        production: impl Fn(&mut Parser) -> Result<Vec<T>, Diagnostic>,
    ) -> Result<Vec<T>, Diagnostic> {
        if !self.at_word(word) {
            return Ok(Vec::new());
        }

        self.advance();

        production(self)
    }

    // `word path_set`, or nothing — the one optional list clause with five call
    // sites, named so those five do not each repeat the production.
    fn optional_path_set(&mut self, word: &str) -> Result<Vec<CellPath>, Diagnostic> {
        self.optional_list(word, Parser::path_set)
    }

    // `word BID`, or nothing — `uses` and `combiner`.
    fn optional_bid(&mut self, word: &str) -> Result<Option<u32>, Diagnostic> {
        if !self.at_word(word) {
            return Ok(None);
        }

        self.advance();

        Ok(Some(self.bid("a behavior handle")?))
    }

    // `word INT`, or nothing, the INT being STRUCTURAL (R57) — `depth` and
    // `history`.
    fn optional_structural_int(
        &mut self,
        word: &str,
        expected: &str,
    ) -> Result<Option<u64>, Diagnostic> {
        if !self.at_word(word) {
            return Ok(None);
        }

        self.advance();
        let (value, _) = self.structural_int(expected)?;

        Ok(Some(value))
    }

    // `word value`, or nothing — `init` and `else`.
    fn optional_value(&mut self, word: &str) -> Result<Option<ValueLiteral>, Diagnostic> {
        if !self.at_word(word) {
            return Ok(None);
        }

        self.advance();

        Ok(Some(self.value()?))
    }

    // A BARE FLAG: the word alone, carrying no payload — `reset_on_reopen` on a
    // cell and on a stream, `reset_on_reload` on a stream (033-215).
    fn flag(&mut self, word: &str) -> bool {
        if !self.at_word(word) {
            return false;
        }

        self.advance();

        true
    }

    // A `BID` (§2): the `u32` the lexer decoded from `B@` and its hex run. The
    // handle is `Copy`, so this reads the value out rather than moving anything.
    fn bid(&mut self, expected: &str) -> Result<u32, Diagnostic> {
        let Some(&TokenKind::Bid(handle)) = self.peek_kind() else {
            return Err(self.unexpected(expected));
        };

        self.advance();

        Ok(handle)
    }

    // A `STRING`'s content, TAKEN out of the token (R18) exactly as a NAME is.
    // The payload is already unescaped by the lexer, so this is the value and
    // not the spelling.
    fn string(&mut self, expected: &str) -> Result<String, Diagnostic> {
        match self.tokens.get_mut(self.cursor) {
            Some(Token {
                kind: TokenKind::Str(text),
                ..
            }) => {
                let value = mem::take(text);
                self.advance();

                Ok(value)
            }
            _ => Err(self.unexpected(expected)),
        }
    }

    // A `QuotedKey`'s content, taken the same way. Also already unescaped: the
    // segment holds what the quotes delimited, and the canonical re-quoting
    // happens where the path is rendered.
    fn quoted_key(&mut self, expected: &str) -> Result<String, Diagnostic> {
        match self.tokens.get_mut(self.cursor) {
            Some(Token {
                kind: TokenKind::QuotedKey(text),
                ..
            }) => {
                let key = mem::take(text);
                self.advance();

                Ok(key)
            }
            _ => Err(self.unexpected(expected)),
        }
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
        self.peek_kind_at(0)
    }

    // What the token `offset` tokens past the cursor is. Exactly one rule
    // passes a non-zero offset, and it passes 1: the COLON rules, through
    // [`peek_int_follows`](Parser::peek_int_follows). Nothing here looks
    // further than one token past the cursor.
    //
    // There used to be a second: the keyword-shadowing lookahead of 033-258,
    // retired when the five spellings became reserved — a path can no longer
    // begin with one, so there is nothing left to disambiguate.
    fn peek_kind_at(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens
            .get(self.cursor + offset)
            .map(|token| &token.kind)
    }

    // Whether an INT stands one token past the cursor — the whole of the
    // colon-ordinal test (§3's boundary note), read without consuming anything.
    fn peek_int_follows(&self) -> bool {
        matches!(self.peek_kind_at(1), Some(TokenKind::Int(_)))
    }

    // The cell kind the word at the cursor names, if it names one. `CellKind`
    // is `Copy`, so this hands back a value and the caller is free to advance.
    fn peek_cell_kind(&self) -> Option<CellKind> {
        match self.peek_kind() {
            Some(TokenKind::Ident(text)) => CellKind::from_word(text),
            _ => None,
        }
    }

    // The payload-free literal the word at the cursor names, if it names one:
    // the four word-shaped `value` alternatives. `-inf` is not among them — it
    // is not word-shaped and arrives as a FLOAT lexeme (033-248).
    fn peek_value_word(&self) -> Option<ValueLiteral> {
        let Some(TokenKind::Ident(text)) = self.peek_kind() else {
            return None;
        };

        let literal = match text.as_str() {
            TRUE => ValueLiteral::True,
            FALSE => ValueLiteral::False,
            NAN => ValueLiteral::Nan,
            INF => ValueLiteral::Inf,
            _ => return None,
        };

        Some(literal)
    }

    // Whether a `value` stands at the cursor rather than a PATH — the one place
    // a binding's two target alternatives are told apart. A number, a string
    // and a value WORD are all unambiguous: the four words are RESERVED
    // (002-39), so no path can begin with one and none of them needs a
    // lookahead to be told from a path (033-258).
    fn at_value_start(&self) -> bool {
        self.peek_value_word().is_some()
            || matches!(
                self.peek_kind(),
                Some(TokenKind::Int(_) | TokenKind::Float(_) | TokenKind::Str(_))
            )
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

    // The span of the token BEFORE the cursor — where an ENTRY's span ends.
    // Every entry trails a run of optional clauses, so its last token is
    // whatever the last clause that matched consumed; asking after the fact
    // beats threading an end span out of every clause reader.
    //
    // Zero-length at offset 0 when the cursor has not moved, which only a
    // hand-built stream reaches.
    fn previous_span(&self) -> Span {
        match self
            .cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
        {
            Some(token) => token.span,
            None => Span::new(0, 0),
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

    // The NAME lexeme (§2): any word-shaped token that is not a RESERVED
    // spelling, taken for its text — literally TAKEN (R18). The payload moves
    // out of the token and into the AST; the emptied token is consumed on the
    // next line and never read again, so the name exists in exactly one place
    // from here on.
    //
    // THE RESERVATION (002-39, 033-259) is checked HERE, once, and that is why
    // it holds everywhere: every name position in the grammar reads its name
    // through this method — a module name, a `type_def` id, a field name, a
    // variant name, a binding name, and a PATH's name segment. `null`, `true`,
    // `false`, `nan` and `inf` are rejected at all six under `reserved_word`,
    // which is its own class because the token IS word-shaped and the position
    // DOES want a word: what is wrong is the spelling.
    //
    // Two consequences ride on it. A PATH can never begin with one of the five,
    // so the two positions admitting both a PATH and a literal need no
    // lookahead (033-258). And a quoted KEY segment is untouched — it never
    // comes through here — so `App.rows.'null'.render_row` stays legal data.
    fn name(&mut self, expected: &str) -> Result<String, Diagnostic> {
        let span = self.span();

        let name = match self.tokens.get_mut(self.cursor) {
            Some(Token {
                kind: TokenKind::Ident(text),
                ..
            }) => mem::take(text),
            _ => return Err(self.unexpected(expected)),
        };

        self.advance();

        if is_reserved_word(&name) {
            return Err(Diagnostic {
                class: DiagClass::ReservedWord,
                span: Some(span),
                path: None,
                message: format!("`{name}` is a reserved spelling and is never a name"),
            });
        }

        Ok(name)
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
    use crate::text::test_support::{
        BLANK_LINES, CANONICAL, ENTRY, GRAPH_ENTRY, ONE_LINE, SCOPE_WORD, WITH_TYPES,
    };
    use crate::text::test_support::{parsed, with_entry, with_scope, with_types};

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
    fn behaviors_admits_no_body_yet() {
        assert_eq!(
            failure("module App {\n  types {}\n  graph {}\n  behaviors { x }\n}\n"),
            ("unexpected_token", Span::new(49, 1))
        );
    }

    // The `graph` section's entry is a `scope` and nothing else, so a word that
    // is not one fails at that word.
    #[test]
    fn the_graph_section_admits_scopes_and_nothing_else() {
        assert_eq!(
            failure("module App {\n  types {}\n  graph { x }\n  behaviors {}\n}\n"),
            ("unexpected_token", Span::new(34, 1))
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

    // ---- graph section ----

    // The two anchors every graph span below is written against, pinned the way
    // the type fixture's are: the scope word at 38, an entry's first byte at 78.
    #[test]
    fn a_scope_parses_with_hand_counted_spans() {
        let source = with_entry("input App.count : i32 init 0");

        assert_eq!(source.find(SCOPE), Some(SCOPE_WORD));
        assert_eq!(source.find("input"), Some(GRAPH_ENTRY));

        let module = parsed(&source);
        let [scope] = module.graph.scopes.as_slice() else {
            panic!("the section holds exactly one scope");
        };

        assert_eq!(scope.path.render(), "App");
        assert!(scope.exposes.is_empty());
        assert!(scope.effects.is_empty());
        assert!(scope.reset_on_reopen.is_empty());
        // `scope` at 38 through the body's `}`: the entry is 28 bytes from 78,
        // its newline is at 106, and the closing `    }` puts `}` at 111.
        assert_eq!(scope.span, Span::new(SCOPE_WORD, 74));
        assert_eq!(source.as_bytes()[111], b'}');

        let [Entry::Cell(cell)] = scope.entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        assert_eq!(cell.path.render(), "App.count");
        // The kind word through the `init` literal — the whole 28-byte entry,
        // its last clause included.
        assert_eq!(cell.span, Span::new(GRAPH_ENTRY, 28));
        // `graph` at 26 through the section's `}` at 115.
        assert_eq!(module.graph.span, Span::new(26, 90));
    }

    // An entry's span ends at whatever its LAST clause consumed. Three shapes
    // that last token can take are pinned here by hand count — a closing `]`,
    // a bare flag word, and a STRING carrying a raw newline; the pinned-span
    // test above covers the fourth, a clause ending in a bare literal.
    //
    // Every count is written against `GRAPH_ENTRY`, the entry's first byte.
    #[test]
    fn an_entrys_span_ends_at_its_last_clauses_last_token() {
        // A SET-final entry: the last token is the closing `]`. Counting the
        // entry: `fold ` 0..5, `App.t ` 5..11, `: ` 11..13, `i32 ` 13..17,
        // `members ` 17..25, `[` 25, `App.a` 26..31, `: ` 31..33,
        // `permanent` 33..42, `]` 42 — 43 bytes.
        let members = with_entry("fold App.t : i32 members [App.a: permanent]");

        assert_eq!(members.find("fold"), Some(GRAPH_ENTRY));
        assert_eq!(entry_span(&members), Span::new(GRAPH_ENTRY, 43));
        assert_eq!(members.as_bytes()[GRAPH_ENTRY + 42], b']');

        // A BARE-FLAG-final entry: the last token is a word carrying no
        // payload. `recurrent ` 0..10, `App.total ` 10..20, `: ` 20..22,
        // `i32 ` 22..26, `reset_on_reopen` 26..41 — 41 bytes.
        let flag = with_entry("recurrent App.total : i32 reset_on_reopen");

        assert_eq!(flag.find("recurrent"), Some(GRAPH_ENTRY));
        assert_eq!(entry_span(&flag), Span::new(GRAPH_ENTRY, 41));

        // The same on a stream, whose last clause may also be a flag:
        // `stream ` 0..7, `App.s ` 7..13, `: ` 13..15, `i32 ` 15..19,
        // `policy ` 19..26, `ring ` 26..31, `capacity ` 31..40, `8 ` 40..42,
        // `reset_on_reload` 42..57 — 57 bytes.
        let stream = with_entry("stream App.s : i32 policy ring capacity 8 reset_on_reload");

        assert_eq!(stream.find("stream"), Some(GRAPH_ENTRY));
        assert_eq!(entry_span(&stream), Span::new(GRAPH_ENTRY, 57));

        // A STRING-payload-final entry whose payload carries a RAW newline
        // (R40): the entry spans two physical lines, because the newline is
        // inside the STRING lexeme and so is not the NEWLINE that terminates an
        // entry (033-250). `input ` 0..6, `App.x ` 6..12, `: ` 12..14,
        // `str ` 14..18, `init ` 18..23, `"a\nb"` 23..28 — 28 bytes, the
        // closing quote included.
        let raw = with_entry("input App.x : str init \"a\nb\"");

        assert_eq!(raw.find("input"), Some(GRAPH_ENTRY));
        assert_eq!(entry_span(&raw), Span::new(GRAPH_ENTRY, 28));
        assert_eq!(raw.as_bytes()[GRAPH_ENTRY + 25], b'\n');
        assert_eq!(raw.as_bytes()[GRAPH_ENTRY + 27], b'"');
    }

    // The span of the one entry a [`with_entry`] fixture's scope holds.
    fn entry_span(source: &str) -> Span {
        let module = parsed(source);

        match module.graph.scopes[0].entries.as_slice() {
            [Entry::Cell(cell)] => cell.span,
            [Entry::Stream(stream)] => stream.span,
            _ => panic!("the scope holds exactly one cell or stream"),
        }
    }

    // 033-256 (FND-16 (b)): the arity is `scope*`, so a module of pure types is
    // inside the grammar rather than tolerated by an unfinished parser.
    #[test]
    fn an_empty_graph_section_is_legal() {
        assert!(parsed(CANONICAL).graph.scopes.is_empty());
        assert!(parsed(ONE_LINE).graph.scopes.is_empty());
        assert!(
            parsed("module App {\n  types {}\n  graph {\n\n  }\n  behaviors {}\n}\n")
                .graph
                .scopes
                .is_empty()
        );
    }

    // Scopes are FLAT under the section: the nesting a program has rides each
    // scope's PATH, so `App.panel` is a sibling here and a child by name.
    #[test]
    fn scopes_are_flat_and_carry_their_nesting_in_their_paths() {
        let module = parsed(&with_scope(
            "scope App exposes [App.count] effects [] {\n    }\n    scope App.panel exposes [] effects [App.panel.print:0] {\n    }",
        ));

        let rendered: Vec<String> = module
            .graph
            .scopes
            .iter()
            .map(|scope| scope.path.render())
            .collect();

        assert_eq!(rendered, ["App", "App.panel"]);
        assert_eq!(module.graph.scopes[0].exposes[0].render(), "App.count");
        assert_eq!(
            module.graph.scopes[1].effects[0].render(),
            "App.panel.print:0"
        );
        // And a scope is not an entry: one written inside a scope body fails at
        // its own keyword.
        assert_eq!(
            failure(&with_entry("scope App.inner exposes [] effects [] {}")),
            ("unexpected_token", Span::new(GRAPH_ENTRY, 5))
        );
    }

    // All five `entry` alternatives in one scope body, each recognized by its
    // leading word.
    #[test]
    fn every_entry_kind_parses() {
        let module = parsed(&with_scope(concat!(
            "scope App exposes [] effects [] {\n",
            "      input App.count : i32 init 0\n",
            "      gate App.g0 pred B@d4 inputs [App.show] guards [App.print:0]\n",
            "      wire App.w from App.a to App.b\n",
            "      effect App.print:0 reconciler \"print\" params [message: App.label]\n",
            "      stream App.s : i32 policy ring capacity 8\n",
            "    }"
        )));

        let entries = &module.graph.scopes[0].entries;

        assert_eq!(entries.len(), 5);
        assert!(matches!(entries[0], Entry::Cell(_)));
        assert!(matches!(entries[1], Entry::Gate(_)));
        assert!(matches!(entries[2], Entry::Wire(_)));
        assert!(matches!(entries[3], Entry::Effect(_)));
        assert!(matches!(entries[4], Entry::Stream(_)));

        let Entry::Gate(gate) = &entries[1] else {
            panic!("the second entry is a gate");
        };

        assert_eq!(gate.path.render(), "App.g0");
        assert_eq!(gate.pred, 0xd4);
        assert_eq!(gate.inputs[0].render(), "App.show");
        assert_eq!(gate.guards[0].render(), "App.print:0");
        assert!(gate.gate_parent.is_none());

        let Entry::Effect(effect) = &entries[3] else {
            panic!("the fourth entry is an effect");
        };

        assert_eq!(effect.reconciler, "print");
        assert_eq!(effect.params.len(), 1);
        assert_eq!(effect.params[0].name, "message");
        assert!(effect.desired.is_empty());
    }

    // Every cell kind, and every clause, in the production's order.
    //
    // The line is ILLEGAL as a program and parses anyway (F4, R21): a `fold`
    // carrying history clauses, and `init` beside `fallbacks` — one clause in
    // two spellings, which the LOADER rejects (033-268) and the grammar does
    // not see.
    #[test]
    fn a_cell_carries_every_clause_in_order() {
        let module = parsed(&with_entry(concat!(
            "fold App.total : i32 uses B@d1 inputs [App.a, App.b] depth 2 ",
            "lookback [App.a: 3] reset_on_reopen init 0 fallbacks [1: 1, 2: 0] ",
            "gate App.g0 combiner B@2 else -1 ",
            "members [App.a: permanent, App.b: gate_guarded]"
        )));

        let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        assert_eq!(cell.kind, CellKind::Fold);
        assert_eq!(cell.path.render(), "App.total");
        assert!(matches!(cell.tag, TypeTag::Prim(Prim::I32)));
        assert_eq!(cell.uses, Some(0xd1));
        assert_eq!(cell.inputs.len(), 2);
        assert_eq!(cell.depth, Some(2));
        assert_eq!(cell.lookback.len(), 1);
        assert_eq!(cell.lookback[0].cell.render(), "App.a");
        assert_eq!(cell.lookback[0].k, 3);
        assert!(cell.reset_on_reopen);
        assert!(matches!(&cell.init, Some(ValueLiteral::Int(text)) if text == "0"));
        assert_eq!(cell.fallbacks.len(), 2);
        assert_eq!(cell.fallbacks[0].k, 1);
        assert!(matches!(&cell.fallbacks[0].value, ValueLiteral::Int(text) if text == "1"));
        assert_eq!(cell.fallbacks[1].k, 2);
        assert!(matches!(&cell.fallbacks[1].value, ValueLiteral::Int(text) if text == "0"));
        assert_eq!(
            cell.gate.as_ref().map(CellPath::render).as_deref(),
            Some("App.g0")
        );
        assert_eq!(cell.combiner, Some(2));
        assert!(matches!(&cell.else_value, Some(ValueLiteral::Int(text)) if text == "-1"));
        assert_eq!(cell.members.len(), 2);
        assert_eq!(cell.members[0].driver, MemberDriver::Permanent);
        assert_eq!(cell.members[1].driver, MemberDriver::GateGuarded);
    }

    #[test]
    fn every_cell_kind_leads_a_line() {
        for (word, kind) in [
            ("input", CellKind::Input),
            ("derived", CellKind::Derived),
            ("recurrent", CellKind::Recurrent),
            ("deferred", CellKind::Deferred),
            ("fold", CellKind::Fold),
        ] {
            let module = parsed(&with_entry(&format!("{word} App.x : i32")));

            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            assert_eq!(cell.kind, kind);
            // An absent set clause and a written empty one are one state.
            assert!(cell.inputs.is_empty());
            assert!(cell.members.is_empty());
        }
    }

    // The clauses are read sequentially in the production's order, so one
    // written out of order does not match at its turn: the entry ends before
    // it, and the terminator rejects it as a second entry on the line.
    #[test]
    fn a_clause_out_of_order_is_rejected_at_that_clause() {
        // `uses` after `inputs`, 35 bytes into the entry.
        assert_eq!(
            failure(&with_entry("derived App.x : i32 inputs [App.y] uses B@d1")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 35, 4))
        );
        // The same clause twice: the second `depth` is 28 bytes in.
        assert_eq!(
            failure(&with_entry("derived App.x : i32 depth 1 depth 2")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 28, 5))
        );
        // `lookback` before `depth`, which the production puts after it: the
        // entry ends at the map's `]` and `depth` is a second entry, 42 bytes
        // in.
        assert_eq!(
            failure(&with_entry(
                "recurrent App.x : i32 lookback [App.a: 1] depth 1"
            )),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 42, 5))
        );
        // `fallbacks` before `init`, the same way round: `init` is 39 bytes in.
        assert_eq!(
            failure(&with_entry("recurrent App.x : i32 fallbacks [1: 0] init 0")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 39, 4))
        );
    }

    // The ordinal attaches to ANY segment (033-130 / 033-133), and the colon
    // that opens it is told from the type-tag colon by what follows: an INT
    // continues the path, anything else does not.
    #[test]
    fn an_ordinal_attaches_to_any_path_segment() {
        let module = parsed(&with_entry(
            "derived App.print:0.text : str uses B@d5 inputs [App.print:12]",
        ));

        let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        assert_eq!(cell.path.render(), "App.print:0.text");
        assert_eq!(cell.inputs[0].render(), "App.print:12");
        assert!(matches!(cell.tag, TypeTag::Prim(Prim::Str)));

        // The structural conversion applies to an ordinal too (R57), so a
        // leading-zero spelling normalizes.
        let zeros = parsed(&with_entry("derived App.print:007 : str"));
        let [Entry::Cell(cell)] = zeros.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        assert_eq!(cell.path.render(), "App.print:7");
    }

    // A quoted key is a path segment of its own (033-243), carrying its
    // UNESCAPED content — and it renders back quoted, since a numeric key is
    // not NAME-shaped (R41).
    #[test]
    fn a_quoted_key_is_a_path_segment() {
        let module = parsed(&with_entry(
            "derived App.rows.'42'.render_row : str inputs [App.rows.'us-east.1'.x]",
        ));

        let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        assert_eq!(cell.path.render(), "App.rows.'42'.render_row");
        assert_eq!(cell.inputs[0].render(), "App.rows.'us-east.1'.x");
        assert!(matches!(cell.path.segments()[2], Segment::Key(_)));
    }

    // §3's boundary note, executable: inside a `member` pair an INT after the
    // colon is the left path's ordinal, and a driver word closes the pair.
    #[test]
    fn a_member_pair_is_told_from_an_ordinal_by_what_follows_the_colon() {
        let module = parsed(&with_entry(
            "fold App.t : i32 members [App.a:0: permanent, App.b: keyed_template]",
        ));

        let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        assert_eq!(cell.members[0].path.render(), "App.a:0");
        assert_eq!(cell.members[0].driver, MemberDriver::Permanent);
        assert_eq!(cell.members[1].path.render(), "App.b");
        assert_eq!(cell.members[1].driver, MemberDriver::KeyedTemplate);
        // An empty set parses; so does the absent clause, and they are one
        // state.
        assert!(
            parsed(&with_entry("fold App.t : i32 members []"))
                .graph
                .scopes[0]
                .entries
                .iter()
                .all(|entry| matches!(entry, Entry::Cell(cell) if cell.members.is_empty()))
        );
    }

    // The same rule on a `reopen_set` pair, which is where §3 states it: after
    // a colon an INT is the left path's ordinal and a NAME begins the right
    // path.
    #[test]
    fn a_reopen_pair_is_told_from_an_ordinal_by_what_follows_the_colon() {
        let module = parsed(&with_scope(
            "scope App exposes [] effects [] reset_on_reopen [App.c:1: App.s, App.d: App.t] {\n    }",
        ));

        let pairs = &module.graph.scopes[0].reset_on_reopen;

        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].consumer.render(), "App.c:1");
        assert_eq!(pairs[0].stream.render(), "App.s");
        assert_eq!(pairs[1].consumer.render(), "App.d");
        assert_eq!(pairs[1].stream.render(), "App.t");
    }

    // R43: in a `lookback_map` entry the LAST `':' INT` is the lookback value,
    // and any earlier one belongs to the key path. Both spellings of that show
    // up here — one where a colon is left over, one where the value was first
    // read as the key's own ordinal and has to be taken back.
    #[test]
    fn a_lookback_entry_takes_its_last_colon_int_as_the_value() {
        let module = parsed(&with_entry(
            "stream App.s : i32 policy ring capacity 8 lookback [App.print:0 : 3, App.a : 5]",
        ));

        let [Entry::Stream(stream)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one stream");
        };

        assert_eq!(stream.lookback.len(), 2);
        assert_eq!(stream.lookback[0].cell.render(), "App.print:0");
        assert_eq!(stream.lookback[0].k, 3);
        assert_eq!(stream.lookback[1].cell.render(), "App.a");
        assert_eq!(stream.lookback[1].k, 5);
    }

    // The taken-back ordinal must be on the LAST segment. `App.print:0.text` is
    // not an entry whose value hides mid-key: its last `':' INT` is inside the
    // path, so the entry has no value and the `:` is what is missing — reported
    // at the `]` that arrived instead, 68 bytes into the entry.
    #[test]
    fn a_lookback_entry_without_a_value_is_rejected() {
        assert_eq!(
            failure(&with_entry(
                "stream App.s : i32 policy ring capacity 8 lookback [App.print:0.text]"
            )),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 68, 1))
        );
    }

    // A `fallback_map` entry has ONE colon and no rule deciding it: the key is
    // an INT, so neither reopen_set's lookahead nor lookback_map's positional
    // law (R43) has anything to settle here.
    //
    // The two halves convert the two opposite ways. `k` is STRUCTURAL and is a
    // `u64` on arrival (R57); the value is TYPED, stays text, and is
    // canonicalized textually where it is built (033-257) — `007` to `7`,
    // `-1.50` to `-1.5`.
    #[test]
    fn a_fallback_map_pairs_a_structural_depth_with_a_canonical_literal() {
        let module = parsed(&with_entry(
            "recurrent App.fib : i64 uses B@d8 depth 7 fallbacks [007: 007, 2: -1.50, 3: \"hi\", 4: nan]",
        ));

        let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        let fallbacks = &cell.fallbacks;

        assert_eq!(fallbacks.len(), 4);
        // The KEY converts by value, so `007` is the number seven.
        assert_eq!(fallbacks[0].k, 7);
        assert!(matches!(&fallbacks[0].value, ValueLiteral::Int(text) if text == "7"));
        assert_eq!(fallbacks[1].k, 2);
        assert!(matches!(&fallbacks[1].value, ValueLiteral::Float(text) if text == "-1.5"));
        assert_eq!(fallbacks[2].k, 3);
        assert!(matches!(&fallbacks[2].value, ValueLiteral::Str(text) if text == "hi"));
        assert_eq!(fallbacks[3].k, 4);
        assert!(matches!(fallbacks[3].value, ValueLiteral::Nan));
    }

    // An absent clause and a written `[]` are one state for BOTH new set
    // clauses, exactly as they are for `inputs` and `members`.
    #[test]
    fn an_empty_history_clause_is_the_clauses_absence() {
        for entry in [
            "recurrent App.x : i32 depth 1",
            "recurrent App.x : i32 depth 1 lookback [] fallbacks []",
        ] {
            let module = parsed(&with_entry(entry));

            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            assert!(cell.lookback.is_empty(), "{entry}");
            assert!(cell.fallbacks.is_empty(), "{entry}");
        }
    }

    #[test]
    fn a_stream_carries_every_clause_in_order() {
        let module = parsed(&with_entry(concat!(
            "stream App.s : i32 policy gate capacity 64 source_deps [App.a] ",
            "observes [App.s.pending_count] reset_on_reload reset_on_reopen ",
            "history 4 lookback [App.a : 2]"
        )));

        let [Entry::Stream(stream)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one stream");
        };

        assert_eq!(stream.path.render(), "App.s");
        assert_eq!(stream.policy, StreamPolicy::Gate);
        assert_eq!(stream.capacity, 64);
        assert_eq!(stream.source_deps[0].render(), "App.a");
        assert_eq!(stream.observes[0].render(), "App.s.pending_count");
        assert!(stream.reset_on_reload);
        assert!(stream.reset_on_reopen);
        assert_eq!(stream.history, Some(4));
        assert_eq!(stream.lookback[0].k, 2);
    }

    // `null` is a literal alternative of the `to` clause, and RESERVED
    // (002-39), so the word is the literal with no lookahead deciding it. What
    // keeps a `null` KEY reachable is quoting, not a path reading — see
    // `a_reserved_spelling_stays_reachable_as_a_quoted_key`.
    #[test]
    fn a_wire_goes_to_a_path_or_to_null() {
        let module = parsed(&with_entry(
            "wire App.w from App.a to null type i32 attrs [k: 1@bridge] gate App.g0",
        ));

        let [Entry::Wire(wire)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one wire");
        };

        assert!(matches!(wire.to, WireTo::Null));
        assert_eq!(wire.from.render(), "App.a");
        assert!(matches!(wire.tag, Some(TypeTag::Prim(Prim::I32))));
        assert_eq!(wire.attrs[0].provenance, Some(Provenance::Bridge));
        assert_eq!(
            wire.gate.as_ref().map(CellPath::render).as_deref(),
            Some("App.g0")
        );

        let path = parsed(&with_entry("wire App.w from App.a to App.b.c"));
        let [Entry::Wire(wire)] = path.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one wire");
        };

        let WireTo::Path(destination) = &wire.to else {
            panic!("the destination is a path");
        };

        assert_eq!(destination.render(), "App.b.c");
    }

    // The reservation costs no expressiveness: a KEY spelled by one of the five
    // is DATA, quotes, and reads back as the key at every path position — the
    // wire destination that admits the keyword included (033-243, 033-259).
    #[test]
    fn a_reserved_spelling_stays_reachable_as_a_quoted_key() {
        let module = parsed(&with_entry(
            "wire App.w from App.'nan'.x to App.rows.'null'.render_row",
        ));

        let [Entry::Wire(wire)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one wire");
        };

        let WireTo::Path(destination) = &wire.to else {
            panic!("the destination is a path");
        };

        assert_eq!(destination.render(), "App.rows.'null'.render_row");
        assert_eq!(wire.from.render(), "App.'nan'.x");
        assert_eq!(
            destination.segments()[2],
            Segment::Key("null".to_string()),
            "the segment holds the UNESCAPED key, not its spelling"
        );

        // A bare `'null'` destination is a one-segment KEY path, not the
        // literal — the quoting is what tells the two apart.
        let keyed = parsed(&with_entry("wire App.w from App.a to 'null'"));
        let [Entry::Wire(wire)] = keyed.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one wire");
        };

        assert!(matches!(&wire.to, WireTo::Path(cell) if cell.render() == "'null'"));
    }

    // A binding's target is a PATH or a value, and the provenance marker is
    // optional metadata on either (033-252).
    #[test]
    fn a_binding_binds_a_path_or_a_value_with_optional_provenance() {
        let module = parsed(&with_entry(concat!(
            "effect App.print:0 reconciler \"print\" params [",
            "message: App.label@bound, count: 7, flag: true, ",
            "text: \"hi\"@constant_wrap, plain: App.other]"
        )));

        let [Entry::Effect(effect)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one effect");
        };

        let [message, count, flag, text, plain] = effect.params.as_slice() else {
            panic!("the effect has five params");
        };

        assert!(
            matches!(&message.target, BindingTarget::Path(path) if path.render() == "App.label")
        );
        assert_eq!(message.provenance, Some(Provenance::Bound));
        assert!(
            matches!(&count.target, BindingTarget::Value(ValueLiteral::Int(text)) if text == "7")
        );
        assert!(matches!(
            flag.target,
            BindingTarget::Value(ValueLiteral::True)
        ));
        assert_eq!(text.provenance, Some(Provenance::ConstantWrap));
        assert!(plain.provenance.is_none());
    }

    // 033-257: a typed value is CANONICALIZED and never converted — the loader
    // owns the type, so the reader normalizes the spelling only.
    #[test]
    fn a_typed_value_is_canonicalized_but_not_converted() {
        for (spelling, canonical) in [
            ("007", "7"),
            ("-0", "0"),
            ("-007", "-7"),
            (
                "170141183460469231731687303715884105727",
                "170141183460469231731687303715884105727",
            ),
        ] {
            let module = parsed(&with_entry(&format!("input App.x : i32 init {spelling}")));
            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            assert!(
                matches!(&cell.init, Some(ValueLiteral::Int(text)) if text == canonical),
                "{spelling}"
            );
        }

        for (spelling, canonical) in [("1.50", "1.5"), ("00.0", "0.0"), ("-0.0", "-0.0")] {
            let module = parsed(&with_entry(&format!("input App.x : f64 init {spelling}")));
            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            assert!(
                matches!(&cell.init, Some(ValueLiteral::Float(text)) if text == canonical),
                "{spelling}"
            );
        }
    }

    // Every `value` alternative, including the three type-gated float
    // spellings — admitted by the grammar wherever a value stands, and left to
    // the loader's type check (033-248).
    #[test]
    fn every_value_alternative_parses() {
        assert!(matches!(&init("5"), ValueLiteral::Int(text) if text == "5"));
        assert!(matches!(&init("1.5"), ValueLiteral::Float(text) if text == "1.5"));
        assert!(matches!(init("true"), ValueLiteral::True));
        assert!(matches!(init("false"), ValueLiteral::False));
        assert!(matches!(&init("\"hi\""), ValueLiteral::Str(text) if text == "hi"));
        assert!(matches!(init("nan"), ValueLiteral::Nan));
        assert!(matches!(init("inf"), ValueLiteral::Inf));
        assert!(matches!(init("-inf"), ValueLiteral::NegInf));
    }

    // The `init` value of the one cell a fixture holds, for the assertions
    // above — the fixture is typed `f64` so the three float spellings sit in a
    // position that would accept them at load, though the grammar admits them
    // anywhere and only the loader checks.
    fn init(spelling: &str) -> ValueLiteral {
        let module = parsed(&with_entry(&format!("input App.x : f64 init {spelling}")));

        let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        cell.init.clone().expect("the cell carries an init")
    }

    // R57 reaches the graph's structural positions too: a depth, a capacity, a
    // history, an ordinal, a lookback value and a fallback KEY are all `u64` at
    // parse time, and a spelling that names none fails at the literal.
    #[test]
    fn a_graph_structural_number_must_name_a_u64() {
        // `depth -1`: the literal is 28 bytes into the entry, 2 bytes long.
        assert_eq!(
            failure(&with_entry("recurrent App.x : i32 depth -1")),
            ("int_out_of_range", Span::new(GRAPH_ENTRY + 28, 2))
        );
        // `capacity` overflowing, 40 bytes in and 20 digits long.
        assert_eq!(
            failure(&with_entry(
                "stream App.s : i32 policy ring capacity 99999999999999999999"
            )),
            ("int_out_of_range", Span::new(GRAPH_ENTRY + 40, 20))
        );
        // An ordinal is structural as well, and the sign lives INSIDE the INT
        // lexeme (033-247) — so `App.p:-1` is a colon followed by an `Int`, the
        // ordinal rule fires, and the failure is the value's, 12 bytes in. Not
        // an `unexpected_token`: the token is exactly the lexeme asked for.
        assert_eq!(
            failure(&with_entry("input App.p:-1 : i32")),
            ("int_out_of_range", Span::new(GRAPH_ENTRY + 12, 2))
        );
        // A fallback KEY is structural while the value beside it is not: the
        // key fails at 33 bytes in, and the `0` after the colon is never asked
        // to be a `u64` at all.
        assert_eq!(
            failure(&with_entry("recurrent App.x : i32 fallbacks [-1: 0]")),
            ("int_out_of_range", Span::new(GRAPH_ENTRY + 33, 2))
        );
    }

    // R59's bound is the `type_tag` production's, so it holds wherever a tag
    // stands — a cell's type included. The tag starts 14 bytes into the entry
    // and each accepted `portal<` is 7 bytes.
    #[test]
    fn the_type_tag_nesting_bound_reaches_a_cell_type() {
        let at_bound = with_entry(&format!("input App.x : {}", portals(TYPE_TAG_DEPTH_LIMIT)));

        assert!(!parsed(&at_bound).graph.scopes[0].entries.is_empty());
        assert_eq!(
            failure(&with_entry(&format!(
                "input App.x : {}",
                portals(TYPE_TAG_DEPTH_LIMIT + 1)
            ))),
            (
                "nesting_too_deep",
                Span::new(GRAPH_ENTRY + 14 + TYPE_TAG_DEPTH_LIMIT * 7, 6)
            )
        );
    }

    // 033-250 at both levels: a NEWLINE terminates a scope's entry, the scope's
    // `}` terminates a final entry sharing its line, and the scope entry itself
    // is terminated by the NEWLINE after that `}`.
    #[test]
    fn entries_and_scopes_are_terminated_by_newlines() {
        // Two entries on one line: the second `input` is 18 bytes in.
        assert_eq!(
            failure(&with_entry("input App.a : i32 input App.b : i32")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 18, 5))
        );
        // Two scopes on one line, rejected at the second `scope`.
        assert_eq!(
            failure(&with_scope(
                "scope App exposes [] effects [] {} scope App.b exposes [] effects [] {}"
            )),
            ("unexpected_token", Span::new(SCOPE_WORD + 35, 5))
        );
        // A final entry may share the scope's closing brace, and a scope may be
        // written entirely on one line.
        let one_line = parsed(&with_scope(
            "scope App exposes [] effects [] { input App.a : i32 }",
        ));

        assert_eq!(one_line.graph.scopes[0].entries.len(), 1);

        // Blank lines inside a scope body are insignificant.
        let spaced = parsed(&with_scope(
            "scope App exposes [] effects [] {\n\n      input App.a : i32\n\n\n      input App.b : i32\n\n    }",
        ));

        assert_eq!(spaced.graph.scopes[0].entries.len(), 2);
    }

    // Each entry kind's own error, at a hand-counted span.
    #[test]
    fn each_entry_kind_reports_at_the_offending_token() {
        // A cell with no `:` before its tag: `i32` stands there, 16 bytes in.
        assert_eq!(
            failure(&with_entry("input App.count i32 init 0")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 16, 3))
        );
        // A gate whose `guards` is misspelled, 40 bytes in.
        assert_eq!(
            failure(&with_entry(
                "gate App.g0 pred B@d4 inputs [App.show] guard [App.print:0]"
            )),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 40, 5))
        );
        // A wire missing its mandatory `to`: the line ending arrives instead,
        // 21 bytes in.
        assert_eq!(
            failure(&with_entry("wire App.w from App.a")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 21, 1))
        );
        // An effect whose reconciler is not a STRING, 30 bytes in.
        assert_eq!(
            failure(&with_entry("effect App.print:0 reconciler print params []")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 30, 5))
        );
        // A stream policy that is neither `ring` nor `gate`, 26 bytes in.
        assert_eq!(
            failure(&with_entry("stream App.s : i32 policy fifo capacity 8")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 26, 4))
        );
        // A word that heads no entry at all.
        assert_eq!(
            failure(&with_entry("banana App.x : i32")),
            ("unexpected_token", Span::new(GRAPH_ENTRY, 6))
        );
        // A provenance marker that is none of the three, 49 bytes in.
        assert_eq!(
            failure(&with_entry(
                "effect App.p reconciler \"r\" params [m: App.label@guessed]"
            )),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 49, 7))
        );
    }

    // R46 where it bites hardest: every clause word and entry head is an
    // ordinary NAME too, so a cell may be called `depth`, `init` or `gate` and
    // a scope may be called `input`. The five RESERVED spellings are the
    // exception and are tested below — soft is the rule, hard is the list.
    #[test]
    fn a_clause_keyword_is_a_legal_cell_name() {
        for word in [DEPTH, INIT, GATE, USES, MEMBERS, STREAM, WIRE, SCOPE] {
            let module = parsed(&with_entry(&format!("input App.{word} : i32")));

            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            assert_eq!(cell.path.render(), format!("App.{word}"));
        }

        let named = parsed(&with_scope("scope input exposes [] effects [] {\n    }"));

        assert_eq!(named.graph.scopes[0].path.render(), "input");
    }

    // The reservation (002-39, 033-259) at EVERY name position the grammar has.
    // One check in `name` is what makes all six hold, so this sweeps all six
    // rather than trusting the one.
    //
    // Spans are hand-counted for the three fixtures whose offsets the
    // `test_support` header pins; the rest assert the class, which is the
    // contract (F3).
    #[test]
    fn a_reserved_spelling_is_never_a_name() {
        // A module name. `module` is 0..6 and the name follows at 7.
        assert_eq!(
            failure("module null { types {} graph {} behaviors {} }"),
            ("reserved_word", Span::new(7, 4))
        );

        // A type id: the `%` sits at ENTRY, so the name starts one byte later.
        assert_eq!(
            failure(&with_types("%nan = record {} size 0 align 1")),
            ("reserved_word", Span::new(ENTRY + 1, 3))
        );

        // A field name, 18 bytes into the entry.
        assert_eq!(
            failure(&with_types("%A = record { inf: i32 } size 0 align 1")),
            ("reserved_word", Span::new(ENTRY + 14, 3))
        );

        // The remaining name positions, by class.
        for (position, source) in [
            (
                "a variant name",
                with_types("%A = enum { #null } size 0 align 1"),
            ),
            (
                "a type id in a tag",
                with_types("%A = record { v: %true } size 0 align 1"),
            ),
            (
                "a binding name",
                with_entry("effect App.p reconciler \"r\" params [false: 1]"),
            ),
            ("a path head", with_entry("input inf.x : i32")),
            ("a path tail", with_entry("input App.null : i32")),
            ("a mid-path segment", with_entry("input App.nan.deep : i32")),
            (
                "a binding's path target",
                with_entry("effect App.p reconciler \"r\" params [m: App.inf]"),
            ),
            (
                "a wire destination",
                with_entry("wire App.w from App.a to App.false"),
            ),
            (
                "a scope path",
                with_scope("scope nan exposes [] effects [] {\n    }"),
            ),
        ] {
            let (class, _) = failure(&source);

            assert_eq!(class, "reserved_word", "{position}");
        }
    }

    // The two positions that admit BOTH a path and a literal are the one place
    // the reservation does not surface as `reserved_word`, and the reason is
    // that the word is not in a name position there at all: it IS the literal,
    // read with no lookahead (033-258). What follows it is then simply the next
    // token, and an unexpected one — which is a rejection all the same, and the
    // one a reader can act on, since `to null` was a complete destination.
    #[test]
    fn a_literal_word_is_taken_before_anything_that_follows_it() {
        // The `.` after `null`, 29 bytes into the entry.
        assert_eq!(
            failure(&with_entry("wire App.w from App.a to null.x")),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 29, 1))
        );
        // The `.` after `true`, 43 bytes in.
        assert_eq!(
            failure(&with_entry(
                "effect App.p reconciler \"r\" params [f: true.state]"
            )),
            ("unexpected_token", Span::new(GRAPH_ENTRY + 43, 1))
        );
    }
}
