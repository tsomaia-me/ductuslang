//! [`emit()`] — the canonical text emitter: the module AST back to `.duir`
//! (R54).

use std::fmt::{Arguments, Write};

use crate::graph::{
    Binding, BindingTarget, Cell, CellKind, Effect, Entry, FallbackEntry, Gate, LookbackEntry,
    Member, ReopenPair, Scope, Stream, ValueLiteral, Wire, WireTo,
};
use crate::module::{GraphSection, Module, TypesSection};
use crate::path::CellPath;
use crate::text::keywords::{
    ALIGN, ATTRS, BEHAVIORS, CAPACITY, CLOSURE, COMBINER, DEPTH, DESIRED, EFFECT, EFFECTS, ELSE,
    ENUM, EXPOSES, FALLBACKS, FALSE, FROM, GATE, GATE_PARENT, GRAPH, GUARDS, HANDLE, HISTORY, INF,
    INIT, INPUTS, LOOKBACK, MEMBERS, MODULE, NAN, NULL, OBSERVED, OBSERVES, PARAMS, POLICY,
    POOL_INDEX, PORTAL, PRED, RECONCILER, RECORD, RESET_ON_RELOAD, RESET_ON_REOPEN, SCOPE, SIZE,
    SOURCE_DEPS, STREAM, TO, TRUE, TUPLE, TYPE, TYPES, USES, WIRE, YIELDED,
};
use crate::text::{is_name_shaped, is_reserved_word, quote};
use crate::types::{Field, Layout, TypeDef, TypeTag, Variant};

/// One level of indentation: two spaces (R54).
const INDENT: &str = "  ";

/// The one list separator: comma, space. Written once so no caller can invent a
/// second spelling of it.
const SEPARATOR: &str = ", ";

/// The one PAIR separator: colon, space. A `field`, a `binding`, a `member`, a
/// `reopen_set` pair, a `lookback_map` entry and a `fallback_map` entry all
/// join their two halves with it.
///
/// A `cell`'s and a `stream`'s `PATH ':' type_tag` is NOT one of these: it is
/// spaced on both sides (`input App.count : i32`), which is what tells a
/// declaration's colon from a pair's at a glance.
const PAIR: &str = ": ";

/// The `BID` prefix (§2), case-sensitive. Not a keyword — the keyword table
/// holds word-shaped spellings only — so it is written here, once.
const BID_PREFIX: &str = "B@";

/// What an omitted `depth` means on a recurrent: one slot (C-7, 016-48).
///
/// The emitter needs it for one reason only — 033-268 picks `init` over
/// `fallbacks` when the map seeds EVERY depth, and "every depth" of a cell that
/// wrote no `depth` clause is this number. It decides a SPELLING here and
/// nothing else; the loader applies the same default to decide the ring.
const DEFAULT_DEPTH: u64 = 1;

/// Render `module` as canonical `.duir` text (IR_GRAMMAR §3, §15.4.6).
///
/// The emitter IS the canonicalizer (R54): there is exactly one rendering of a
/// given module, so this function is also the definition of module equivalence
/// (R17) — two modules are the same when their emitted text is.
///
/// The canonical form of a module with one type row:
///
/// ```text
/// module App {
///   types {
///     %Request = record { text: str } size 8 align 8
///   }
///   graph {}
///   behaviors {}
/// }
/// ```
///
/// Two spaces per indent level, one construct per line, sections in grammar
/// order, an empty section as `name {}` with nothing between its braces, and
/// exactly one trailing newline.
///
/// Within a line, the canon is: a single space between tokens that need one,
/// `, ` between list elements, a NON-EMPTY brace body spaced inside
/// (`record { text: str }`) and an empty one closed up (`record {}`), and type
/// tags GLUED — `pool_index<%Request>`, `handle<portal<i32>>`, `[i32; 4]`,
/// `tuple(i32, f64)`. The one interior space that is not a separator is the one
/// around a closure's arrow: `closure<(i32) -> str>`.
///
/// # What `emit` assumes about the module it is handed
///
/// Emitting is total over the modules its PRODUCERS build — not over every
/// `Module` value that can be written down. The producers are the parser and
/// (later) the loader, and both build a `Module` out of text that has already
/// been lexed, so `Module::name` is NAME-shaped (§2) and the round trip
/// `emit(parse(text))` re-parses.
///
/// A hand-built `Module` can break that. `Module` is plain data with public
/// fields and no constructor by design — legality is the loader's alone (F4) —
/// so nothing stops a caller writing `name: "not a name".to_string()`, and the
/// text that would emit does not lex. That is a producer bug, not an emitter
/// case: `emit` `debug_assert`s what the grammar guarantees, at the place each
/// claim reaches the output, and otherwise trusts what it was given.
///
/// The guard covers every NAME position and every ARITY position:
///
/// - every NAME — a module name, a `type_def` id, a field name, a variant name,
///   a binding name, and the id inside a `%NAME` or `pool_index<%NAME>` tag —
///   is NAME-shaped AND is not one of the five reserved spellings (002-39,
///   033-259). Both conjuncts are one law: what the lexer hands a NAME position
///   is charset-legal and never `null`, `true`, `false`, `nan` or `inf`, so a
///   guard checking the charset alone would pass `module null { … }` — text
///   the parser rejects as `reserved_word`. A PATH is exempt because it guards
///   itself: `CellPath::new` asserts both claims on every segment it is built
///   from, so a path reaching here has already been checked;
/// - every ARITY the grammar makes non-empty — an `enum` layout's variants, a
///   `tuple` layout's elements, a tuple TAG's elements — is non-empty. (A
///   `record`'s field list and a `closure`'s parameter list are optional in the
///   grammar, so neither is asserted.)
///
/// There used to be a third family: a path written where a literal WORD is also
/// admitted, guarded against being that bare word. The RESERVATION retired it
/// (002-39, 033-259) — `null`, `true`, `false`, `nan` and `inf` are never a
/// `NAME`, so `CellPath::new` rejects such a segment before any path reaches
/// here, and the guard would be a second check of a claim already made. A
/// quoted KEY spelled by one of the five is a different thing entirely: it is
/// data, and [`CellPath::render`](crate::path::CellPath::render) keeps it
/// quoted so it reads back as the key.
///
/// It does NOT check the third grammar constraint, the 64-level `type_tag`
/// nesting bound (033-255): the emitter recurses per level exactly as the
/// parser does, so asserting it here would trade a rendering for a panic on a
/// tag that only a hand-built module can hold. A hand-built tag deeper than the
/// bound therefore emits text that `parse` rejects — which is a producer bug of
/// the same family as a name that does not lex, and is why round-trip totality
/// is stated as a claim about PARSED modules and nothing wider.
///
/// `debug_assert` compiles out in release, where a violated claim renders
/// whatever it renders — `enum {}` for the empty enum, say — rather than
/// costing every well-formed module a check.
#[must_use]
pub fn emit(module: &Module) -> String {
    let mut emitter = Emitter {
        out: String::new(),
        indent: 0,
    };

    emitter.module(module);

    emitter.out
}

/// The text so far and the level it is being written at — the two things a
/// render method cannot derive from the AST node it was handed.
struct Emitter {
    out: String,
    indent: usize,
}

impl Emitter {
    // module ::= 'module' NAME '{' types_section graph_section behaviors_section '}'
    fn module(&mut self, module: &Module) {
        // The producer contract, checked where the name reaches the output.
        debug_assert!(
            is_name_shaped(&module.name) && !is_reserved_word(&module.name),
            "a module name is NAME-shaped and never reserved: {}",
            module.name
        );

        self.block(format_args!("{MODULE} {}", module.name), |emitter| {
            emitter.types_section(&module.types);
            emitter.graph_section(&module.graph);
            emitter.behaviors_section();
        });
    }

    // types_section ::= 'types' '{' type_def* '}'
    //
    // An EMPTY table is one line — `types {}` — rather than an opened block
    // with nothing in it. Same rule as the other two sections, and the reason
    // the minimal module reads the way it does.
    fn types_section(&mut self, types: &TypesSection) {
        if types.type_defs.is_empty() {
            self.line(&format!("{TYPES} {{}}"));

            return;
        }

        self.block(format_args!("{TYPES}"), |emitter| {
            for type_def in &types.type_defs {
                emitter.type_def(type_def);
            }
        });
    }

    // type_def ::= '%' NAME '=' layout 'size' INT 'align' INT
    //
    // One row, one line — which is what 033-250's entry terminator means on the
    // writing side: the emitter never puts two rows on a line, so its output
    // never needs the reader to disambiguate one.
    //
    // The row is written straight into the sink, layout and all: a `type_def`
    // is the one construct whose middle is a nested rendering, and there is no
    // reason for that middle to become a `String` on the way out.
    fn type_def(&mut self, type_def: &TypeDef) {
        // The producer contract, checked where the id reaches the output.
        debug_assert!(
            is_name_shaped(&type_def.id) && !is_reserved_word(&type_def.id),
            "a type id is NAME-shaped and never reserved: {}",
            type_def.id
        );

        self.indentation();
        let _ = write!(self.out, "%{} = ", type_def.id);
        layout(&mut self.out, &type_def.layout);
        let _ = writeln!(
            self.out,
            " {SIZE} {} {ALIGN} {}",
            type_def.size, type_def.align
        );
    }

    // graph_section ::= 'graph' '{' scope* '}'
    //
    // Empty is one line — `graph {}` — the same rule the other two sections
    // follow, and the form 033-256 makes legal rather than tolerated. A
    // non-empty section opens a block holding one scope per line-group.
    fn graph_section(&mut self, graph: &GraphSection) {
        if graph.scopes.is_empty() {
            self.line(&format!("{GRAPH} {{}}"));

            return;
        }

        self.block(format_args!("{GRAPH}"), |emitter| {
            for scope in &graph.scopes {
                emitter.scope(scope);
            }
        });
    }

    // scope ::= 'scope' PATH 'exposes' path_set 'effects' path_set
    //           ('reset_on_reopen' reopen_set)? '{' entry* '}'
    //
    // `exposes` and `effects` are MANDATORY, so an empty one renders as a
    // written `[]`; `reset_on_reopen` is optional, so an empty one renders as
    // nothing at all. That is the whole difference between the two kinds of set
    // clause, and it is decided here rather than inside the set renderer.
    //
    // A scope with no entries closes on its own line — `scope … {}` — for the
    // same reason an empty section does: an opened block with nothing in it is
    // two lines spelling what one spells.
    fn scope(&mut self, scope: &Scope) {
        let mut header = String::new();

        header.push_str(SCOPE);
        header.push(' ');
        path(&mut header, &scope.path);
        clause(&mut header, EXPOSES);
        path_set(&mut header, &scope.exposes);
        clause(&mut header, EFFECTS);
        path_set(&mut header, &scope.effects);
        optional_set(
            &mut header,
            RESET_ON_REOPEN,
            &scope.reset_on_reopen,
            reopen_set,
        );

        if scope.entries.is_empty() {
            // The closed-up form, written straight into the sink: `header` is
            // already a `String`, and formatting it into a second one only to
            // copy that into the output is one allocation and one copy too
            // many. Same shape as `block`'s opening line.
            self.indentation();
            let _ = writeln!(self.out, "{header} {{}}");

            return;
        }

        self.block(format_args!("{header}"), |emitter| {
            for entry in &scope.entries {
                emitter.entry(entry);
            }
        });
    }

    // entry ::= cell | gate | wire | effect | stream
    //
    // One entry, one line — 033-250's terminator on the writing side. The
    // indentation and the line ending are written HERE, so the five renderers
    // below write a line BODY and none of them can forget either half.
    fn entry(&mut self, entry: &Entry) {
        self.indentation();

        match entry {
            Entry::Cell(cell) => self.cell(cell),
            Entry::Gate(gate) => self.gate(gate),
            Entry::Wire(wire) => self.wire(wire),
            Entry::Effect(effect) => self.effect(effect),
            Entry::Stream(stream) => self.stream(stream),
        }

        self.out.push('\n');
    }

    // cell ::= cell_kind PATH ':' type_tag
    //          ('uses' BID)? ('inputs' path_set)? ('depth' INT)?
    //          ('lookback' lookback_map)?
    //          ('reset_on_reopen')? ('init' value)?
    //          ('fallbacks' fallback_map)? ('gate' PATH)?
    //          ('combiner' BID)? ('else' value)? ('members' member_set)?
    //
    // The clauses are written in the production's order, which is the order the
    // parser reads them in — a clause written anywhere else would not parse
    // back. Each optional SET clause is omitted when empty, because an absent
    // clause and a written `[]` are one state on a cell and omission is that
    // state's canonical rendering.
    //
    // Which clauses a KIND admits is not the emitter's question either (F4,
    // R21): a hand-built `input` cell carrying a `combiner` renders it, and the
    // loader is what rejects the module.
    //
    // `init` VERSUS `fallbacks` is the one clause pair whose canonical SPELLING
    // is a function of the cell's VALUES rather than of its shape alone. On a
    // recurrent the two are one clause in two spellings — `init v` says every
    // depth falls back to `v`, the map says them one by one — and 033-268 picks
    // between them BY CONTENT: `init` when every depth shares one value,
    // `fallbacks` otherwise. So a map that seeds every depth with one value is
    // the NON-canonical spelling of `init v`, and it is written `init v` here,
    // exactly as `inputs []` is written by being omitted.
    //
    // [`uniform_seed`] is that rule and nothing beyond it. Everything it
    // declines renders as the AST holds it — both clauses at once, a map with a
    // hole, a key out of range, `fallbacks` on a kind that admits none — because
    // each of those means something the loader refuses (F4, R21), and an AST
    // that means two things is not the emitter's to repair.
    fn cell(&mut self, cell: &Cell) {
        let out = &mut self.out;

        out.push_str(cell.kind.as_str());
        out.push(' ');
        path(out, &cell.path);
        out.push_str(" : ");
        type_tag(out, &cell.tag);
        optional_bid(out, USES, cell.uses);
        optional_set(out, INPUTS, &cell.inputs, path_set);
        optional_int(out, DEPTH, cell.depth);
        optional_set(out, LOOKBACK, &cell.lookback, lookback_map);
        flag(out, RESET_ON_REOPEN, cell.reset_on_reopen);

        if let Some(seed) = uniform_seed(cell) {
            clause(out, INIT);
            value(out, seed);
        } else {
            optional_value(out, INIT, cell.init.as_ref());
            optional_set(out, FALLBACKS, &cell.fallbacks, fallback_map);
        }

        optional_path(out, GATE, cell.gate.as_ref());
        optional_bid(out, COMBINER, cell.combiner);
        optional_value(out, ELSE, cell.else_value.as_ref());
        optional_set(out, MEMBERS, &cell.members, member_set);
    }

    // gate ::= 'gate' PATH 'pred' BID 'inputs' path_set 'guards' path_set
    //          ('gate_parent' PATH)?
    //
    // `inputs` and `guards` are MANDATORY here, unlike a cell's `inputs`: an
    // empty one is written `[]` rather than omitted. What the grammar requires
    // is always written.
    fn gate(&mut self, gate: &Gate) {
        let out = &mut self.out;

        out.push_str(GATE);
        out.push(' ');
        path(out, &gate.path);
        clause(out, PRED);
        bid(out, gate.pred);
        clause(out, INPUTS);
        path_set(out, &gate.inputs);
        clause(out, GUARDS);
        path_set(out, &gate.guards);
        optional_path(out, GATE_PARENT, gate.gate_parent.as_ref());
    }

    // wire ::= 'wire' PATH 'from' PATH 'to' (PATH | 'null')
    //          ('type' type_tag)? ('attrs' binding_set)? ('gate' PATH)?
    fn wire(&mut self, wire: &Wire) {
        let out = &mut self.out;

        out.push_str(WIRE);
        out.push(' ');
        path(out, &wire.path);
        clause(out, FROM);
        path(out, &wire.from);
        clause(out, TO);

        // No guard on the PATH arm: `null` is reserved, so `CellPath::new` has
        // already rejected a segment spelled that way and no path reaching here
        // can render as the literal (002-39, 033-258).
        match &wire.to {
            WireTo::Path(destination) => path(out, destination),
            WireTo::Null => out.push_str(NULL),
        }

        if let Some(tag) = &wire.tag {
            clause(out, TYPE);
            type_tag(out, tag);
        }

        optional_set(out, ATTRS, &wire.attrs, binding_set);
        optional_path(out, GATE, wire.gate.as_ref());
    }

    // effect ::= 'effect' PATH 'reconciler' STRING 'params' binding_set
    //            ('desired' path_set)? ('observed' path_set)? ('gate' PATH)?
    //
    // The reconciler NAME is held as a value — escapes already processed — so
    // its spelling is derived here, by the same canonical escaper a `str`
    // value's is.
    fn effect(&mut self, effect: &Effect) {
        let out = &mut self.out;

        out.push_str(EFFECT);
        out.push(' ');
        path(out, &effect.path);
        clause(out, RECONCILER);
        string(out, &effect.reconciler);
        clause(out, PARAMS);
        binding_set(out, &effect.params);
        optional_set(out, DESIRED, &effect.desired, path_set);
        optional_set(out, OBSERVED, &effect.observed, path_set);
        optional_path(out, GATE, effect.gate.as_ref());
    }

    // stream ::= 'stream' PATH ':' type_tag 'policy' ('ring' | 'gate') 'capacity' INT
    //            ('source_deps' path_set)? ('observes' path_set)?
    //            ('reset_on_reload')? ('reset_on_reopen')?
    //            ('history' INT)? ('lookback' lookback_map)?
    fn stream(&mut self, stream: &Stream) {
        let out = &mut self.out;

        out.push_str(STREAM);
        out.push(' ');
        path(out, &stream.path);
        out.push_str(" : ");
        type_tag(out, &stream.tag);
        let _ = write!(
            out,
            " {POLICY} {} {CAPACITY} {}",
            stream.policy.as_str(),
            stream.capacity
        );
        optional_set(out, SOURCE_DEPS, &stream.source_deps, path_set);
        optional_set(out, OBSERVES, &stream.observes, path_set);
        flag(out, RESET_ON_RELOAD, stream.reset_on_reload);
        flag(out, RESET_ON_REOPEN, stream.reset_on_reopen);
        optional_int(out, HISTORY, stream.history);
        optional_set(out, LOOKBACK, &stream.lookback, lookback_map);
    }

    // behaviors_section ::= 'behaviors' '{' behavior* '}'
    fn behaviors_section(&mut self) {
        self.line(&format!("{BEHAVIORS} {{}}"));
    }

    // A braced block: `header {`, the body one level deeper, then `}`. The
    // indent level is raised and lowered HERE and nowhere else, so a body that
    // returns early — or forgets to restore the level — is not a thing that can
    // be written. Every nested construct from increment 5 on renders through
    // this.
    //
    // `header` arrives as `fmt::Arguments` so a caller can interpolate into the
    // opening line without allocating a `String` to hand over.
    fn block(&mut self, header: Arguments<'_>, body: impl FnOnce(&mut Emitter)) {
        self.indentation();
        // Writing into a `String` is infallible; the `Result` is there for
        // `fmt::Write`'s io-backed implementors.
        let _ = writeln!(self.out, "{header} {{");

        self.indent += 1;
        body(self);
        self.indent -= 1;

        self.line("}");
    }

    // One output line: the current indent, the text, and the line ending.
    fn line(&mut self, text: &str) {
        self.indentation();
        self.out.push_str(text);
        self.out.push('\n');
    }

    // The current level's leading whitespace. Both line writers open with it,
    // and neither writes anything else before the line's own text.
    fn indentation(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str(INDENT);
        }
    }
}

// layout ::= 'record' '{' field_list '}'
//          | 'enum'   '{' variant (',' variant)* '}'
//          | 'tuple'  '(' type_tag (',' type_tag)* ')'
//
// A layout renders INSIDE a line rather than as one, because a `type_def` is
// one line and a layout is the middle of it. So this and everything below take
// the line's text as a sink and append to it: a nested tag is written where it
// belongs rather than built as a `String` and handed back up, which is what
// keeps a deep tag from allocating once per level.
fn layout(out: &mut String, layout: &Layout) {
    match layout {
        Layout::Record(fields) => braced(out, RECORD, fields.is_empty(), |out| {
            field_list(out, fields);
        }),
        Layout::Enum(variants) => {
            debug_assert!(
                !variants.is_empty(),
                "an `enum` layout carries at least one variant"
            );

            braced(out, ENUM, variants.is_empty(), |out| {
                variant_list(out, variants);
            });
        }
        Layout::Tuple(elements) => {
            debug_assert!(
                !elements.is_empty(),
                "a `tuple` layout carries at least one element"
            );

            out.push_str(TUPLE);
            out.push('(');
            tag_list(out, elements);
            out.push(')');
        }
    }
}

// field_list ::= (NAME ':' type_tag (',' NAME ':' type_tag)*)?
fn field_list(out: &mut String, fields: &[Field]) {
    separated(out, fields, field);
}

// `NAME ':' type_tag` — one element of a `field_list`.
fn field(out: &mut String, field: &Field) {
    // The producer contract, checked where the name reaches the output.
    debug_assert!(
        is_name_shaped(&field.name) && !is_reserved_word(&field.name),
        "a field name is NAME-shaped and never reserved: {}",
        field.name
    );

    out.push_str(&field.name);
    out.push_str(PAIR);
    type_tag(out, &field.tag);
}

// `variant (',' variant)*` — an `enum` layout's cases, in declaration order.
fn variant_list(out: &mut String, variants: &[Variant]) {
    separated(out, variants, variant);
}

// variant ::= '#' NAME ('(' type_tag (',' type_tag)* ')')?
//
// No payload, no parentheses: `#None`, never `#None()` — which is also the only
// spelling the grammar admits, so the round trip has one form to preserve.
fn variant(out: &mut String, variant: &Variant) {
    debug_assert!(
        is_name_shaped(&variant.name) && !is_reserved_word(&variant.name),
        "a variant name is NAME-shaped and never reserved: {}",
        variant.name
    );

    out.push('#');
    out.push_str(&variant.name);

    if variant.payload.is_empty() {
        return;
    }

    out.push('(');
    tag_list(out, &variant.payload);
    out.push(')');
}

// type_tag ::= PRIM | '%' NAME | 'pool_index' '<' '%' NAME '>' | …
//
// Recursive, mirroring the parser: each alternative writes its own shape and
// asks this function for whatever it wraps. Tags are GLUED — no space inside
// `<…>`, `(…)` or `[…]` — with two exceptions that are separators rather than
// padding: the `, ` between list elements and the ` -> ` of a closure.
fn type_tag(out: &mut String, tag: &TypeTag) {
    match tag {
        TypeTag::Prim(prim) => out.push_str(prim.as_str()),
        TypeTag::TypeId(id) => {
            debug_assert!(
                is_name_shaped(id) && !is_reserved_word(id),
                "a type id is NAME-shaped and never reserved: {id}"
            );

            out.push('%');
            out.push_str(id);
        }
        TypeTag::PoolIndex(id) => {
            debug_assert!(
                is_name_shaped(id) && !is_reserved_word(id),
                "a type id is NAME-shaped and never reserved: {id}"
            );

            let _ = write!(out, "{POOL_INDEX}<%{id}>");
        }
        TypeTag::Yielded(inner) => wrapper(out, YIELDED, inner),
        TypeTag::Handle(inner) => wrapper(out, HANDLE, inner),
        TypeTag::Portal(inner) => wrapper(out, PORTAL, inner),
        TypeTag::Tuple(elements) => {
            debug_assert!(
                !elements.is_empty(),
                "a tuple tag carries at least one element"
            );

            out.push('(');
            tag_list(out, elements);
            out.push(')');
        }
        TypeTag::Array(element, length) => {
            out.push('[');
            type_tag(out, element);
            let _ = write!(out, "; {length}]");
        }
        TypeTag::Closure { params, ret } => {
            let _ = write!(out, "{CLOSURE}<(");
            tag_list(out, params);
            out.push_str(") -> ");
            type_tag(out, ret);
            out.push('>');
        }
    }
}

// `keyword '<' type_tag '>'` — the single-argument wrapper shape `yielded`,
// `handle` and `portal` share, mirroring the parser's `angled_tag`.
fn wrapper(out: &mut String, keyword: &str, inner: &TypeTag) {
    out.push_str(keyword);
    out.push('<');
    type_tag(out, inner);
    out.push('>');
}

// `type_tag (',' type_tag)*` — the tag run shared by tuples, variant payloads
// and closure parameter lists.
fn tag_list(out: &mut String, tags: &[TypeTag]) {
    separated(out, tags, type_tag);
}

// member_set ::= '[' (member (',' member)*)? ']'
fn member_set(out: &mut String, members: &[Member]) {
    bracketed(out, members, member);
}

// member ::= PATH ':' member_driver
//
// The pair's colon is written CLOSED UP against the path (`App.a: permanent`),
// which is also what keeps it readable next to an ordinal-suffixed key:
// `App.a:0: permanent` is the cell `App.a:0` driven `permanent`.
fn member(out: &mut String, member: &Member) {
    path(out, &member.path);
    out.push_str(PAIR);
    out.push_str(member.driver.as_str());
}

// path_set ::= '[' (PATH (',' PATH)*)? ']'
fn path_set(out: &mut String, paths: &[CellPath]) {
    bracketed(out, paths, path);
}

// binding_set ::= '[' (binding (',' binding)*)? ']'
fn binding_set(out: &mut String, bindings: &[Binding]) {
    bracketed(out, bindings, binding);
}

// binding ::= NAME ':' (PATH | value) ('@' provenance)?
//
// The provenance marker is GLUED to the target — `message: App.label@bound` —
// because it is metadata ON the binding rather than a clause of its own
// (033-252). The `@` is what separates them; a space would read as two things.
fn binding(out: &mut String, binding: &Binding) {
    // The producer contract, checked where the name reaches the output.
    debug_assert!(
        is_name_shaped(&binding.name) && !is_reserved_word(&binding.name),
        "a binding name is NAME-shaped and never reserved: {}",
        binding.name
    );

    out.push_str(&binding.name);
    out.push_str(PAIR);

    // No guard on the PATH arm, for the reason a wire's destination has none:
    // the four value words are reserved, so no path can be named by one
    // (002-39, 033-258).
    match &binding.target {
        BindingTarget::Path(target) => path(out, target),
        BindingTarget::Value(literal) => value(out, literal),
    }

    if let Some(provenance) = binding.provenance {
        out.push('@');
        out.push_str(provenance.as_str());
    }
}

// reopen_set ::= '[' (PATH ':' PATH (',' PATH ':' PATH)*)? ']'
fn reopen_set(out: &mut String, pairs: &[ReopenPair]) {
    bracketed(out, pairs, reopen_pair);
}

// `PATH ':' PATH` — one `(consumer : stream)` pair of a `reopen_set`.
fn reopen_pair(out: &mut String, pair: &ReopenPair) {
    path(out, &pair.consumer);
    out.push_str(PAIR);
    path(out, &pair.stream);
}

// lookback_map ::= '[' (PATH ':' INT (',' PATH ':' INT)*)? ']'
fn lookback_map(out: &mut String, entries: &[LookbackEntry]) {
    bracketed(out, entries, lookback_entry);
}

// `PATH ':' INT` — one `(input cell : k)` pair, written the way R43 reads it:
// the entry's LAST `':' INT` is the value. `App.a: 5` re-reads as the cell
// `App.a` at lookback 5 (the `:5` is taken back out of the key), and
// `App.print:0: 3` as the cell `App.print:0` at lookback 3.
fn lookback_entry(out: &mut String, entry: &LookbackEntry) {
    path(out, &entry.cell);
    out.push_str(PAIR);
    let _ = write!(out, "{}", entry.k);
}

// fallback_map ::= '[' (INT ':' value (',' INT ':' value)*)? ']'
fn fallback_map(out: &mut String, entries: &[FallbackEntry]) {
    bracketed(out, entries, fallback_entry);
}

// `INT ':' value` — one `(depth : fallback)` pair, and the one pair production
// with no colon question at all: the key is an INT, so `1: 0` has exactly one
// colon and reads back the only way it can.
fn fallback_entry(out: &mut String, entry: &FallbackEntry) {
    let _ = write!(out, "{}", entry.k);
    out.push_str(PAIR);
    value(out, &entry.value);
}

// The value `init` spells for this cell, or `None` when `init` is not the
// canonical spelling of what the cell holds — 033-268's by-content pick.
//
// It answers `Some` on exactly one shape: a RECURRENT carrying `fallbacks` and
// no `init`, whose keys cover `1 ..= depth` once each, and whose values are all
// one value. That is the map that MEANS `init v`, so `init v` is what it is
// written as. Every other shape is `None` and renders unchanged — a non-recurrent
// kind, both clauses at once, a hole, a duplicate or out-of-range key, or values
// that differ — because each is either the general form or a module the loader
// refuses, and neither is repaired here (F4, R21).
//
// Value equality is by CANONICAL TEXT, which is what `ValueLiteral` equality is
// (R17's note): the literals were normalized where they were built (033-257), so
// `[1: 007, 2: 7]` arrives here as two `7`s and folds, while `0.0` and `-0.0`
// stay two values, exactly as their renderings say.
fn uniform_seed(cell: &Cell) -> Option<&ValueLiteral> {
    if cell.kind != CellKind::Recurrent || cell.init.is_some() || cell.fallbacks.is_empty() {
        return None;
    }

    if !covers_each_depth(&cell.fallbacks, cell.depth.unwrap_or(DEFAULT_DEPTH)) {
        return None;
    }

    let first = &cell.fallbacks[0].value;
    let mut rendered = String::new();
    let mut seed = String::new();

    value(&mut seed, first);

    for entry in &cell.fallbacks[1..] {
        rendered.clear();
        value(&mut rendered, &entry.value);

        if rendered != seed {
            return None;
        }
    }

    Some(first)
}

// Whether `entries` seeds every depth of a `depth`-deep history exactly once —
// the totality the loader demands (`missing_fallback_key`) and the emitter needs
// before it may fold a map into `init`.
//
// The map's OWN length is tested first, so a `depth` of a billion with three
// entries costs one comparison and no allocation.
fn covers_each_depth(entries: &[FallbackEntry], depth: u64) -> bool {
    if u64::try_from(entries.len()) != Ok(depth) {
        return false;
    }

    let mut seeded = vec![false; entries.len()];

    for entry in entries {
        let Ok(k) = usize::try_from(entry.k) else {
            return false;
        };

        if k < 1 || k > seeded.len() || seeded[k - 1] {
            return false;
        }

        seeded[k - 1] = true;
    }

    true
}

// PATH ::= PathSegment ('.' PathSegment)*
//
// The rendering is `CellPath`'s own and is canonical there (R41): a key is
// bare iff it is NAME-shaped and single-quoted otherwise. It writes INTO the
// line being built — `render_into`, not `render` — so no path costs a `String`
// of its own on the way out, and this exists so a path can also be an ELEMENT
// of the bracketed lists above, which take a renderer of exactly this shape.
fn path(out: &mut String, path: &CellPath) {
    path.render_into(out);
}

// value ::= INT | FLOAT | 'true' | 'false' | STRING | 'nan' | 'inf' | '-inf'
//
// The digits are already canonical when they arrive: `ValueLiteral::int` and
// `ValueLiteral::float` normalize the spelling where the literal is BUILT
// (033-257), so this writes the stored text as it stands and no second
// normalization can disagree with the first. A `str` is the exception — it
// holds the VALUE, so its one canonical spelling is derived here.
fn value(out: &mut String, literal: &ValueLiteral) {
    match literal {
        ValueLiteral::Int(text) | ValueLiteral::Float(text) => out.push_str(text),
        ValueLiteral::True => out.push_str(TRUE),
        ValueLiteral::False => out.push_str(FALSE),
        ValueLiteral::Str(content) => string(out, content),
        ValueLiteral::Nan => out.push_str(NAN),
        ValueLiteral::Inf => out.push_str(INF),
        // `-inf` is one lexeme, not a negation — the sign lives inside it
        // (033-247) — and it is the one float spelling that is not word-shaped.
        ValueLiteral::NegInf => {
            out.push('-');
            out.push_str(INF);
        }
    }
}

// A `STRING`'s canonical spelling (R40, 033-249) — the `quote` law at the `"`
// delimiter.
//
// The escaping itself lives at `text/` level, because the `QuotedKey` emitter
// writes the SAME law at the `'` delimiter (033-243, R63): one payload grammar,
// one canonical spelling of it, one implementation. This wrapper stays because
// this file's private functions are named after the productions they write, and
// `STRING` is a production.
fn string(out: &mut String, content: &str) {
    quote(out, '"', content);
}

// BID ::= 'B@' HexU32
//
// LOWERCASE hex with no leading zeros — `B@d1`, `B@aa10`. The handle IS the
// `u32` and the hex is a spelling of it, so the canonical spelling is the
// shortest run that names it; `{:x}` is exactly that.
fn bid(out: &mut String, handle: u32) {
    let _ = write!(out, "{BID_PREFIX}{handle:x}");
}

// `keyword { body }`, or `keyword {}` when the body is empty — the brace
// spacing rule in the one place it is decided.
//
// An empty `record` is the case that matters. An empty `enum` is
// ungrammatical: the parser cannot produce one, and `layout` `debug_assert`s
// against a hand-built one — so `empty` there is the release-mode fallback,
// rendering `enum {}` rather than the `enum {  }` an unconditional body would
// write.
fn braced(out: &mut String, keyword: &str, empty: bool, body: impl FnOnce(&mut String)) {
    out.push_str(keyword);

    if empty {
        out.push_str(" {}");

        return;
    }

    out.push_str(" { ");
    body(out);
    out.push_str(" }");
}

// The separator-writing loop every list production shares: `SEPARATOR` before
// every element but the first, then the element's own rendering.
fn separated<T>(out: &mut String, items: &[T], render: impl Fn(&mut String, &T)) {
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push_str(SEPARATOR);
        }

        render(out, item);
    }
}

// `'[' (element (',' element)*)? ']'` — the bracketed comma list that six
// productions share: `path_set`, `binding_set`, `member_set`, `reopen_set`,
// `lookback_map` and `fallback_map`. The brackets and the empty case are
// written once, exactly as the parser reads them once.
fn bracketed<T>(out: &mut String, items: &[T], render: impl Fn(&mut String, &T)) {
    out.push('[');
    separated(out, items, render);
    out.push(']');
}

// ` word ` — a clause's leading space, its keyword, and the space before its
// payload. Every clause opens through this, so none of them can invent its own
// spacing.
fn clause(out: &mut String, word: &str) {
    out.push(' ');
    out.push_str(word);
    out.push(' ');
}

// ` word`, or nothing — a BARE FLAG, which carries no payload and so takes no
// trailing space: `reset_on_reopen` on a cell and on a stream,
// `reset_on_reload` on a stream (033-215).
fn flag(out: &mut String, word: &str, present: bool) {
    if !present {
        return;
    }

    out.push(' ');
    out.push_str(word);
}

// ` word PATH`, or nothing — the optional single-path clause shared by `gate`
// (on a cell, a wire and an effect) and `gate_parent`.
fn optional_path(out: &mut String, word: &str, cell: Option<&CellPath>) {
    let Some(cell) = cell else {
        return;
    };

    clause(out, word);
    path(out, cell);
}

// ` word INT`, or nothing, the INT being a STRUCTURAL number written back from
// the `u64` it was converted to (R57) — `depth` and `history`. The conversion
// is why a leading-zero spelling does not survive a round trip: the number
// round-trips, and the canonical digits are what comes out.
fn optional_int(out: &mut String, word: &str, number: Option<u64>) {
    let Some(number) = number else {
        return;
    };

    clause(out, word);
    let _ = write!(out, "{number}");
}

// ` word BID`, or nothing — `uses` and `combiner`.
fn optional_bid(out: &mut String, word: &str, handle: Option<u32>) {
    let Some(handle) = handle else {
        return;
    };

    clause(out, word);
    bid(out, handle);
}

// ` word value`, or nothing — `init` and `else`.
fn optional_value(out: &mut String, word: &str, literal: Option<&ValueLiteral>) {
    let Some(literal) = literal else {
        return;
    };

    clause(out, word);
    value(out, literal);
}

// ` word '[' … ']'`, or nothing — every OPTIONAL set clause: a cell's `inputs`,
// `lookback`, `fallbacks` and `members`, a wire's `attrs`, an effect's
// `desired` and `observed`, a stream's `source_deps`, `observes` and
// `lookback`, and a scope's `reset_on_reopen`.
//
// Empty means ABSENT for all of them, so an empty set writes nothing at all —
// which is how `inputs []` canonicalizes to omission. The MANDATORY sets — a
// scope's `exposes` and `effects`, a gate's `inputs` and `guards`, an effect's
// `params` — do not come through here: they are written unconditionally, `[]`
// included.
//
// `render` is the SET production's own renderer, not its element's, so a call
// site names the production it is writing.
fn optional_set<T>(out: &mut String, word: &str, items: &[T], render: impl Fn(&mut String, &[T])) {
    if items.is_empty() {
        return;
    }

    clause(out, word);
    render(out, items);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Span;
    use crate::graph::{CellKind, MemberDriver, Provenance, StreamPolicy};
    use crate::module::{BehaviorsSection, TypesSection};
    use crate::path::Segment;
    use crate::text::is_reserved_word;
    use crate::text::test_support::{BLANK_LINES, CANONICAL, ONE_LINE, WITH_TYPES};
    use crate::text::test_support::{parsed, with_entry, with_types};
    use crate::types::Prim;
    use proptest::prelude::*;

    // A copy of `corpus/duir/m0/types_table.duir`, so the canon this file pins
    // and the canon the corpus witness pins are the same text. The FILE is the
    // authority — `ductus-conformance`'s `types_table` test reads it from disk
    // and makes the same assertion — and a drift between the two costs nothing
    // here, since a fixpoint assertion holds of whatever text it is given.
    const WITNESS: &str = "module App {
  types {
    %Payload = record {} size 0 align 1
    %Request = record { text: str, body: pool_index<%Payload>, tags: [u8; 4] } size 32 align 8
    %Status = enum { #Ok(i32, str), #Pending, #Failed(%Request) } size 24 align 8
    %Pair = tuple(i32, f64) size 16 align 8
    %Port = record { slot: handle<portal<i32>>, apply: closure<(i32, str) -> bool> } size 24 align 8
    %View = record { rows: [%Status; 8], empty: closure<() -> str>, nested: (bool, (f32, i128)) } size 40 align 8
  }
  graph {}
  behaviors {}
}
";

    // A copy of the graph lines the four `corpus/duir/m0` witnesses pin —
    // `scopes`, `cells`, `graph_entries` and `recurrent_clauses` — packed into
    // one module, so the canon this file pins and the canon the corpus pins are
    // the same text. The FILES are the authority, as with `WITNESS` above.
    const GRAPH_WITNESS: &str = "module App {
  types {}
  graph {
    scope App exposes [App.count] effects [App.print:0] reset_on_reopen [App.mixer.tap: App.events] {
      input App.count : i32 init 0
      input App.label : str init \"say \\\"hi\\\"\"
      derived App.doubled : i32 uses B@d1 inputs [App.count]
      recurrent App.total : i32 uses B@d2 inputs [App.count] depth 1 reset_on_reopen init 0
      recurrent App.fib : i64 uses B@d8 depth 2 fallbacks [1: 1, 2: 0]
      recurrent App.avg : f64 uses B@d7 inputs [App.x] depth 2 lookback [App.x: 3] reset_on_reopen fallbacks [1: 0.0, 2: -1.5] gate App.g0
      recurrent App.blend : f64 uses B@d9 inputs [App.print:0] depth 1 lookback [App.print:0: 2, App.x: 1] init nan
      fold App.sum : i32 gate App.g0 combiner B@aa10 else -1 members [App.a: permanent, App.rows.'42'.render_row: keyed_template, App.rows.'it\\'s'.render_row: keyed_template, App.rows.'null'.render_row: keyed_template, App.c: gate_guarded]
      gate App.g0 pred B@d4 inputs [App.show] guards [App.print:0] gate_parent App.g1
      wire App.w0 from App.a to null type i32 attrs [k: 1@bridge, m: App.label@bound] gate App.g0
      effect App.print:0 reconciler \"print\" params [message: App.label@bound] desired [App.print:0.text] observed [App.print:0.ok]
      stream App.events : f64 policy gate capacity 8 source_deps [App.source] reset_on_reload history 4 lookback [App.print:0: 3, App.a: 5]
    }
    scope App.panel exposes [] effects [] {}
  }
  behaviors {}
}
";

    // The line-interior renderers write into a sink rather than returning one,
    // so a test that asserts on one construct's text supplies the sink.
    fn rendered(render: impl FnOnce(&mut String)) -> String {
        let mut out = String::new();
        render(&mut out);

        out
    }

    // One entry as the emitter writes it, without the indentation and the line
    // ending `entry` wraps it in — the five renderers write a line BODY, and
    // that body is what a canon assertion is about.
    fn entry_line(entry: &Entry) -> String {
        let mut emitter = Emitter {
            out: String::new(),
            indent: 0,
        };

        emitter.entry(entry);

        emitter
            .out
            .strip_suffix('\n')
            .expect("an entry ends its line")
            .to_string()
    }

    // Spans play no part in the rendering, so a hand-built fixture carries a
    // zero-length one: the fixture is about the TEXT and not about where a
    // diagnostic would point.
    fn nowhere() -> Span {
        Span::new(0, 0)
    }

    fn name(text: &str) -> Segment {
        Segment::Name {
            text: text.to_string(),
            ordinal: None,
        }
    }

    fn cell_path(segments: &[&str]) -> CellPath {
        CellPath::new(segments.iter().map(|text| name(text)).collect())
    }

    // `App.<text>:<index>` — the ordinal-suffixed path an anonymous instance
    // gets.
    fn ordinal_path(text: &str, index: u64) -> CellPath {
        CellPath::new(vec![
            name("App"),
            Segment::Name {
                text: text.to_string(),
                ordinal: Some(index),
            },
        ])
    }

    // A module whose `graph` section holds exactly `scopes` and whose other two
    // sections are empty.
    fn with_graph(scopes: Vec<Scope>) -> Module {
        Module {
            name: "App".to_string(),
            span: nowhere(),
            types: TypesSection {
                span: nowhere(),
                type_defs: Vec::new(),
            },
            graph: GraphSection {
                span: nowhere(),
                scopes,
            },
            behaviors: BehaviorsSection { span: nowhere() },
        }
    }

    // A scope named `path`, its mandatory sets empty, holding `entries`.
    fn scope_of(path: CellPath, entries: Vec<Entry>) -> Scope {
        Scope {
            path,
            span: nowhere(),
            exposes: Vec::new(),
            effects: Vec::new(),
            reset_on_reopen: Vec::new(),
            entries,
        }
    }

    // An `input App.count : i32` cell with every optional clause absent — the
    // fixture the clause tests vary one field of.
    fn bare_cell() -> Cell {
        Cell {
            kind: CellKind::Input,
            path: cell_path(&["App", "count"]),
            span: nowhere(),
            tag: TypeTag::Prim(Prim::I32),
            uses: None,
            inputs: Vec::new(),
            depth: None,
            lookback: Vec::new(),
            reset_on_reopen: false,
            init: None,
            fallbacks: Vec::new(),
            gate: None,
            combiner: None,
            else_value: None,
            members: Vec::new(),
        }
    }

    // Spans play no part in the rendering; these are the canonical source's,
    // so the fixture reads as the module that source parses to.
    #[test]
    fn a_minimal_module_renders_in_canonical_form() {
        let module = Module {
            name: "App".to_string(),
            span: Span::new(0, 51),
            types: TypesSection {
                span: Span::new(15, 8),
                type_defs: Vec::new(),
            },
            graph: GraphSection {
                span: Span::new(26, 8),
                scopes: Vec::new(),
            },
            behaviors: BehaviorsSection {
                span: Span::new(37, 12),
            },
        };

        assert_eq!(emit(&module), CANONICAL);
    }

    // A non-empty table opens a block; an empty one stays on its line. The
    // fixture is hand-built rather than parsed, so the rendering is asserted
    // against the AST and not against a source that already looks like it.
    #[test]
    fn a_type_table_renders_one_row_per_line() {
        let module = Module {
            name: "App".to_string(),
            span: Span::new(0, 105),
            types: TypesSection {
                span: Span::new(15, 62),
                type_defs: vec![TypeDef {
                    id: "Request".to_string(),
                    span: Span::new(27, 46),
                    layout: Layout::Record(vec![Field {
                        name: "text".to_string(),
                        tag: TypeTag::Prim(Prim::Str),
                    }]),
                    size: 8,
                    align: 8,
                }],
            },
            graph: GraphSection {
                span: Span::new(80, 8),
                scopes: Vec::new(),
            },
            behaviors: BehaviorsSection {
                span: Span::new(91, 12),
            },
        };

        assert_eq!(emit(&module), WITH_TYPES);
    }

    // The line-interior canon, one assertion per form. These are what the
    // corpus witness is checked against, written out rather than derived.
    #[test]
    fn every_layout_renders_in_canonical_form() {
        assert_eq!(
            rendered(|out| layout(out, &Layout::Record(Vec::new()))),
            "record {}"
        );
        assert_eq!(
            rendered(|out| layout(
                out,
                &Layout::Record(vec![
                    Field {
                        name: "text".to_string(),
                        tag: TypeTag::Prim(Prim::Str),
                    },
                    Field {
                        name: "count".to_string(),
                        tag: TypeTag::Prim(Prim::I32),
                    },
                ])
            )),
            "record { text: str, count: i32 }"
        );
        assert_eq!(
            rendered(|out| layout(
                out,
                &Layout::Enum(vec![
                    Variant {
                        name: "Some".to_string(),
                        payload: vec![TypeTag::Prim(Prim::I32), TypeTag::Prim(Prim::Str)],
                    },
                    Variant {
                        name: "None".to_string(),
                        payload: Vec::new(),
                    },
                ])
            )),
            "enum { #Some(i32, str), #None }"
        );
        assert_eq!(
            rendered(|out| layout(
                out,
                &Layout::Tuple(vec![TypeTag::Prim(Prim::I32), TypeTag::Prim(Prim::F64)])
            )),
            "tuple(i32, f64)"
        );
    }

    #[test]
    fn every_type_tag_renders_in_canonical_form() {
        assert_eq!(
            rendered(|out| type_tag(out, &TypeTag::Prim(Prim::Isize))),
            "isize"
        );
        assert_eq!(
            rendered(|out| type_tag(out, &TypeTag::TypeId("Request".to_string()))),
            "%Request"
        );
        assert_eq!(
            rendered(|out| type_tag(out, &TypeTag::PoolIndex("Request".to_string()))),
            "pool_index<%Request>"
        );
        assert_eq!(
            rendered(|out| type_tag(out, &TypeTag::Yielded(Box::new(TypeTag::Prim(Prim::Bool))))),
            "yielded<bool>"
        );
        assert_eq!(
            rendered(|out| type_tag(
                out,
                &TypeTag::Handle(Box::new(TypeTag::Portal(Box::new(TypeTag::Prim(
                    Prim::I32
                )))))
            )),
            "handle<portal<i32>>"
        );
        assert_eq!(
            rendered(|out| type_tag(
                out,
                &TypeTag::Tuple(vec![TypeTag::Prim(Prim::I32), TypeTag::Prim(Prim::Str)])
            )),
            "(i32, str)"
        );
        assert_eq!(
            rendered(|out| type_tag(out, &TypeTag::Array(Box::new(TypeTag::Prim(Prim::I32)), 4))),
            "[i32; 4]"
        );
        assert_eq!(
            rendered(|out| type_tag(
                out,
                &TypeTag::Closure {
                    params: vec![TypeTag::Prim(Prim::I32)],
                    ret: Box::new(TypeTag::Prim(Prim::Str)),
                }
            )),
            "closure<(i32) -> str>"
        );
        assert_eq!(
            rendered(|out| type_tag(
                out,
                &TypeTag::Closure {
                    params: Vec::new(),
                    ret: Box::new(TypeTag::Prim(Prim::Str)),
                }
            )),
            "closure<() -> str>"
        );
    }

    // The graph section's two forms and the scope's two, in one module: a
    // section holding scopes opens a block, a scope holding entries opens a
    // block, and a scope holding none closes up on its line.
    #[test]
    fn a_graph_section_renders_one_scope_per_line_group() {
        let mut app = scope_of(cell_path(&["App"]), vec![Entry::Cell(bare_cell())]);
        app.exposes = vec![cell_path(&["App", "count"])];
        app.reset_on_reopen = vec![ReopenPair {
            consumer: cell_path(&["App", "c"]),
            stream: cell_path(&["App", "s"]),
        }];

        let module = with_graph(vec![
            app,
            scope_of(cell_path(&["App", "panel"]), Vec::new()),
        ]);

        assert_eq!(
            emit(&module),
            concat!(
                "module App {\n",
                "  types {}\n",
                "  graph {\n",
                "    scope App exposes [App.count] effects [] reset_on_reopen [App.c: App.s] {\n",
                "      input App.count : i32\n",
                "    }\n",
                "    scope App.panel exposes [] effects [] {}\n",
                "  }\n",
                "  behaviors {}\n",
                "}\n"
            )
        );
    }

    // Every clause the `cell` production admits, in the production's order —
    // the same line `parse`'s clause test reads, asserted from the other side.
    //
    // The line is deliberately ILLEGAL as a program: a `fold` carrying `depth`
    // and history clauses, and `init` standing beside `fallbacks`. Both are the
    // loader's to reject (F4, R21, 033-268), and rendering them is how the
    // emitter shows it canonicalizes spellings rather than repairing meaning.
    #[test]
    fn a_cell_renders_every_clause_in_the_productions_order() {
        let cell = Cell {
            kind: CellKind::Fold,
            path: cell_path(&["App", "total"]),
            span: nowhere(),
            tag: TypeTag::Prim(Prim::I32),
            uses: Some(0xd1),
            inputs: vec![cell_path(&["App", "a"]), cell_path(&["App", "b"])],
            depth: Some(2),
            lookback: vec![LookbackEntry {
                cell: cell_path(&["App", "a"]),
                k: 3,
            }],
            reset_on_reopen: true,
            init: Some(ValueLiteral::int("0".to_string())),
            fallbacks: vec![
                FallbackEntry {
                    k: 1,
                    value: ValueLiteral::int("1".to_string()),
                },
                FallbackEntry {
                    k: 2,
                    value: ValueLiteral::int("0".to_string()),
                },
            ],
            gate: Some(cell_path(&["App", "g0"])),
            combiner: Some(2),
            else_value: Some(ValueLiteral::int("-1".to_string())),
            members: vec![
                Member {
                    path: cell_path(&["App", "a"]),
                    driver: MemberDriver::Permanent,
                },
                Member {
                    path: cell_path(&["App", "b"]),
                    driver: MemberDriver::GateGuarded,
                },
            ],
        };

        assert_eq!(
            entry_line(&Entry::Cell(cell)),
            concat!(
                "fold App.total : i32 uses B@d1 inputs [App.a, App.b] depth 2 ",
                "lookback [App.a: 3] reset_on_reopen init 0 fallbacks [1: 1, 2: 0] ",
                "gate App.g0 combiner B@2 else -1 ",
                "members [App.a: permanent, App.b: gate_guarded]"
            )
        );
    }

    // The other side of the same production: every clause absent. An empty
    // OPTIONAL set is the clause's absence, so it writes nothing — `inputs []`
    // is a spelling the parser accepts and the emitter never produces.
    #[test]
    fn a_cells_absent_clauses_and_empty_sets_write_nothing() {
        assert_eq!(
            entry_line(&Entry::Cell(bare_cell())),
            "input App.count : i32"
        );

        for kind in [
            CellKind::Input,
            CellKind::Derived,
            CellKind::Recurrent,
            CellKind::Deferred,
            CellKind::Fold,
        ] {
            let mut cell = bare_cell();
            cell.kind = kind;

            assert_eq!(
                entry_line(&Entry::Cell(cell)),
                format!("{} App.count : i32", kind.as_str())
            );
        }
    }

    // A gate's `inputs` and `guards` are MANDATORY, so an empty one is written
    // `[]` — the distinction a cell's optional `inputs` does not have.
    #[test]
    fn a_gate_writes_its_mandatory_sets_even_when_empty() {
        let full = Gate {
            path: cell_path(&["App", "g0"]),
            span: nowhere(),
            pred: 0xd4,
            inputs: vec![cell_path(&["App", "show"])],
            guards: vec![ordinal_path("print", 0)],
            gate_parent: Some(cell_path(&["App", "g1"])),
        };

        assert_eq!(
            entry_line(&Entry::Gate(full)),
            "gate App.g0 pred B@d4 inputs [App.show] guards [App.print:0] gate_parent App.g1"
        );

        let empty = Gate {
            path: cell_path(&["App", "g0"]),
            span: nowhere(),
            pred: 0,
            inputs: Vec::new(),
            guards: Vec::new(),
            gate_parent: None,
        };

        assert_eq!(
            entry_line(&Entry::Gate(empty)),
            "gate App.g0 pred B@0 inputs [] guards []"
        );
    }

    // `null` is an alternative the text spells out, not an omitted clause, so
    // the `to` clause is written either way.
    #[test]
    fn a_wire_renders_a_path_or_a_null_destination() {
        let full = Wire {
            path: cell_path(&["App", "w"]),
            span: nowhere(),
            from: cell_path(&["App", "a"]),
            to: WireTo::Path(cell_path(&["App", "b"])),
            tag: Some(TypeTag::Prim(Prim::I32)),
            attrs: vec![
                Binding {
                    name: "k".to_string(),
                    target: BindingTarget::Value(ValueLiteral::int("1".to_string())),
                    provenance: Some(Provenance::Bridge),
                },
                Binding {
                    name: "m".to_string(),
                    target: BindingTarget::Path(cell_path(&["App", "label"])),
                    provenance: None,
                },
            ],
            gate: Some(cell_path(&["App", "g0"])),
        };

        assert_eq!(
            entry_line(&Entry::Wire(full)),
            "wire App.w from App.a to App.b type i32 attrs [k: 1@bridge, m: App.label] gate App.g0"
        );

        let null = Wire {
            path: cell_path(&["App", "w"]),
            span: nowhere(),
            from: cell_path(&["App", "a"]),
            to: WireTo::Null,
            tag: None,
            attrs: Vec::new(),
            gate: None,
        };

        assert_eq!(
            entry_line(&Entry::Wire(null)),
            "wire App.w from App.a to null"
        );
    }

    #[test]
    fn an_effect_renders_its_reconciler_as_a_canonical_string() {
        let full = Effect {
            path: ordinal_path("print", 0),
            span: nowhere(),
            reconciler: "print".to_string(),
            params: vec![Binding {
                name: "message".to_string(),
                target: BindingTarget::Path(cell_path(&["App", "label"])),
                provenance: Some(Provenance::Bound),
            }],
            desired: vec![CellPath::new(vec![
                name("App"),
                Segment::Name {
                    text: "print".to_string(),
                    ordinal: Some(0),
                },
                name("text"),
            ])],
            observed: vec![cell_path(&["App", "ok"])],
            gate: Some(cell_path(&["App", "g0"])),
        };

        assert_eq!(
            entry_line(&Entry::Effect(full)),
            concat!(
                "effect App.print:0 reconciler \"print\" ",
                "params [message: App.label@bound] desired [App.print:0.text] ",
                "observed [App.ok] gate App.g0"
            )
        );

        // `params` is mandatory, so an empty one is `[]`; a reconciler name is
        // a value, so the empty one has a spelling too.
        let empty = Effect {
            path: cell_path(&["App", "p"]),
            span: nowhere(),
            reconciler: String::new(),
            params: Vec::new(),
            desired: Vec::new(),
            observed: Vec::new(),
            gate: None,
        };

        assert_eq!(
            entry_line(&Entry::Effect(empty)),
            "effect App.p reconciler \"\" params []"
        );
    }

    #[test]
    fn a_stream_renders_every_clause_in_the_productions_order() {
        let full = Stream {
            path: cell_path(&["App", "s"]),
            span: nowhere(),
            tag: TypeTag::Prim(Prim::I32),
            policy: StreamPolicy::Gate,
            capacity: 64,
            source_deps: vec![cell_path(&["App", "a"])],
            observes: vec![cell_path(&["App", "s", "pending_count"])],
            reset_on_reload: true,
            reset_on_reopen: true,
            history: Some(4),
            lookback: vec![LookbackEntry {
                cell: cell_path(&["App", "a"]),
                k: 2,
            }],
        };

        assert_eq!(
            entry_line(&Entry::Stream(full)),
            concat!(
                "stream App.s : i32 policy gate capacity 64 source_deps [App.a] ",
                "observes [App.s.pending_count] reset_on_reload reset_on_reopen ",
                "history 4 lookback [App.a: 2]"
            )
        );

        let minimal = Stream {
            path: cell_path(&["App", "s"]),
            span: nowhere(),
            tag: TypeTag::Prim(Prim::I32),
            policy: StreamPolicy::Ring,
            capacity: 8,
            source_deps: Vec::new(),
            observes: Vec::new(),
            reset_on_reload: false,
            reset_on_reopen: false,
            history: None,
            lookback: Vec::new(),
        };

        assert_eq!(
            entry_line(&Entry::Stream(minimal)),
            "stream App.s : i32 policy ring capacity 8"
        );
    }

    // A behavior handle is a `u32` and the hex is a spelling of it, so the
    // canonical spelling is the shortest run that names it: lowercase, no
    // leading zeros.
    #[test]
    fn a_behavior_handle_renders_in_minimal_lowercase_hex() {
        assert_eq!(rendered(|out| bid(out, 0)), "B@0");
        assert_eq!(rendered(|out| bid(out, 0xd1)), "B@d1");
        assert_eq!(rendered(|out| bid(out, 0xaa10)), "B@aa10");
        assert_eq!(rendered(|out| bid(out, u32::MAX)), "B@ffffffff");
    }

    #[test]
    fn every_value_alternative_renders_in_canonical_form() {
        // The digits are canonicalized where the literal is BUILT (033-257),
        // so the emitter writes what it was given and the two cannot disagree.
        assert_eq!(
            rendered(|out| value(out, &ValueLiteral::int("007".to_string()))),
            "7"
        );
        assert_eq!(
            rendered(|out| value(out, &ValueLiteral::int("-0".to_string()))),
            "0"
        );
        assert_eq!(
            rendered(|out| value(out, &ValueLiteral::float("1.50".to_string()))),
            "1.5"
        );
        assert_eq!(
            rendered(|out| value(out, &ValueLiteral::float("-0.0".to_string()))),
            "-0.0"
        );
        assert_eq!(rendered(|out| value(out, &ValueLiteral::True)), "true");
        assert_eq!(rendered(|out| value(out, &ValueLiteral::False)), "false");
        assert_eq!(rendered(|out| value(out, &ValueLiteral::Nan)), "nan");
        assert_eq!(rendered(|out| value(out, &ValueLiteral::Inf)), "inf");
        assert_eq!(rendered(|out| value(out, &ValueLiteral::NegInf)), "-inf");
        assert_eq!(
            rendered(|out| value(out, &ValueLiteral::Str("hi".to_string()))),
            "\"hi\""
        );
    }

    // R40's canonical emission, one assertion per rule: raw wherever raw is
    // legal, four exceptions, and LF and TAB raw among them.
    #[test]
    fn a_string_renders_in_its_one_canonical_spelling() {
        assert_eq!(rendered(|out| string(out, "print")), "\"print\"");
        assert_eq!(rendered(|out| string(out, "")), "\"\"");
        assert_eq!(rendered(|out| string(out, "say \"hi\"")), r#""say \"hi\"""#);
        assert_eq!(rendered(|out| string(out, "a\\b")), r#""a\\b""#);
        // A raw CR would not survive file-level newline normalization.
        assert_eq!(rendered(|out| string(out, "a\rb")), r#""a\rb""#);
        // A newline and a tab emit RAW — the readability half of the design.
        assert_eq!(rendered(|out| string(out, "a\nb\tc")), "\"a\nb\tc\"");
        // Every other control scalar escapes, the C1 block included.
        assert_eq!(
            rendered(|out| string(out, "\u{0}\u{7}\u{1f}\u{7f}\u{85}")),
            r#""\u{0}\u{7}\u{1f}\u{7f}\u{85}""#
        );
        // A non-control scalar is raw whatever its width.
        assert_eq!(rendered(|out| string(out, "é 日")), "\"é 日\"");
    }

    // TOTALITY and UNIQUENESS, executable: the canonical spelling of a value
    // reads back AS that value, and emitting the result changes nothing.
    #[test]
    fn every_string_spelling_reads_back_as_its_own_value() {
        for content in [
            "",
            "print",
            "say \"hi\"",
            "a\\b",
            "a\rb",
            "a\nb\tc",
            "\u{0}\u{7f}\u{85}",
            "é 日",
        ] {
            let spelled = rendered(|out| string(out, content));
            let source = with_entry(&format!("input App.x : str init {spelled}"));
            let module = parsed(&source);

            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            assert!(
                matches!(&cell.init, Some(ValueLiteral::Str(text)) if text == content),
                "{content:?}"
            );
            assert_eq!(emit(&module), source, "{content:?}");
        }
    }

    // An ordinal-bearing key in a PAIR position, where the pair's own separator
    // is also a colon: `App.a:0: permanent` is the cell `App.a:0` driven
    // `permanent`. The emitter writes the pair colon closed up, and the round
    // trip is what proves that spelling reads back as the same two halves.
    #[test]
    fn an_ordinal_bearing_member_key_round_trips() {
        let source =
            with_entry("fold App.t : i32 members [App.a:0: permanent, App.b: keyed_template]");

        assert_eq!(emit(&parsed(&source)), source);
    }

    // The same shape in a `reopen_set`, whose BOTH halves are paths — so an
    // ordinal can sit on either side of the pair colon and the reading is still
    // §3's: after a colon an INT continues the left path and a word begins the
    // right one.
    #[test]
    fn an_ordinal_bearing_reopen_consumer_round_trips() {
        let source = concat!(
            "module App {\n",
            "  types {}\n",
            "  graph {\n",
            "    scope App exposes [] effects [] reset_on_reopen [App.c:1: App.s, App.d: App.t:2] {}\n",
            "  }\n",
            "  behaviors {}\n",
            "}\n"
        );

        assert_eq!(emit(&parsed(source)), source);
    }

    // `-inf` is the one float spelling that is not word-shaped (033-248), so it
    // travels as a FLOAT lexeme and lands on its own variant. Both positions a
    // `value` stands in get the full trip: a cell's `init` and a binding's
    // target.
    #[test]
    fn negative_infinity_round_trips_in_both_value_positions() {
        let init = with_entry("input App.x : f64 init -inf");

        assert_eq!(emit(&parsed(&init)), init);

        let binding = with_entry("effect App.p reconciler \"r\" params [limit: -inf]");

        assert_eq!(emit(&parsed(&binding)), binding);
    }

    // One stream carrying EVERY clause the production admits, through both
    // directions at once: the parsed shape is asserted field by field and the
    // text it emits back is the text it came from. The clause-order tests above
    // assert each direction against a hand-built fixture; this asserts the two
    // agree.
    #[test]
    fn a_stream_carrying_every_clause_round_trips() {
        let source = with_entry(concat!(
            "stream App.s : i32 policy gate capacity 64 source_deps [App.a] ",
            "observes [App.s.pending_count] reset_on_reload reset_on_reopen ",
            "history 4 lookback [App.print:0: 3, App.a: 5]"
        ));
        let module = parsed(&source);

        let [Entry::Stream(stream)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one stream");
        };

        assert_eq!(stream.policy, StreamPolicy::Gate);
        assert_eq!(stream.capacity, 64);
        assert_eq!(stream.source_deps.len(), 1);
        assert_eq!(stream.observes.len(), 1);
        assert!(stream.reset_on_reload);
        assert!(stream.reset_on_reopen);
        assert_eq!(stream.history, Some(4));
        // R43 both ways round: a key that keeps its ordinal, and one whose
        // trailing `:5` is taken back out of the key as the value.
        assert_eq!(stream.lookback[0].cell.render(), "App.print:0");
        assert_eq!(stream.lookback[0].k, 3);
        assert_eq!(stream.lookback[1].cell.render(), "App.a");
        assert_eq!(stream.lookback[1].k, 5);
        assert_eq!(emit(&module), source);
    }

    // The two history clauses, through the same two rounds — the one place a
    // cell's clause list gained a SET whose elements are pairs.
    //
    // R43 shows up here too, because `lookback` on a cell is the same
    // production it is on a stream: `App.print:0: 2` keeps its ordinal and
    // `App.x: 1` gives its trailing `:1` up as the value.
    #[test]
    fn a_cell_carrying_both_history_clauses_round_trips() {
        let source = with_entry(concat!(
            "recurrent App.avg : f64 uses B@d7 inputs [App.x] depth 2 ",
            "lookback [App.print:0: 2, App.x: 1] reset_on_reopen ",
            "fallbacks [1: 0.0, 2: -1.5]"
        ));
        let module = parsed(&source);

        let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
            panic!("the scope holds exactly one cell");
        };

        assert_eq!(cell.depth, Some(2));
        assert_eq!(cell.lookback[0].cell.render(), "App.print:0");
        assert_eq!(cell.lookback[0].k, 2);
        assert_eq!(cell.lookback[1].cell.render(), "App.x");
        assert_eq!(cell.lookback[1].k, 1);
        assert_eq!(cell.fallbacks.len(), 2);
        assert_eq!(cell.fallbacks[0].k, 1);
        assert_eq!(cell.fallbacks[1].k, 2);
        assert_eq!(emit(&module), source);
    }

    // The MANDATORY positions at their smallest: a handle of zero, two empty
    // sets that are still written `[]`, and the empty reconciler name — every
    // one of them a spelling that exists only because the grammar requires the
    // position to be filled.
    #[test]
    fn the_smallest_mandatory_spellings_round_trip() {
        let gate = with_entry("gate App.g0 pred B@0 inputs [] guards []");

        assert_eq!(emit(&parsed(&gate)), gate);

        let effect = with_entry("effect App.p reconciler \"\" params []");

        assert_eq!(emit(&parsed(&effect)), effect);
    }

    #[test]
    fn emitting_a_parsed_module_reproduces_its_canonical_source() {
        assert_eq!(emit(&parsed(CANONICAL)), CANONICAL);
        assert_eq!(emit(&parsed(WITH_TYPES)), WITH_TYPES);
    }

    // The emitter is the canonicalizer: a legal but non-canonical source
    // normalizes to the one true rendering.
    #[test]
    fn a_non_canonical_source_normalizes() {
        assert_eq!(emit(&parsed(ONE_LINE)), CANONICAL);
        assert_eq!(emit(&parsed(BLANK_LINES)), CANONICAL);
        // A type row written the way IR_GRAMMAR §5 writes it — one line, extra
        // spacing — is legal and normalizes to the canonical rendering.
        assert_eq!(
            emit(&parsed(
                "module App { types { %Request  =  record{text:str}   size 8   align 8 } graph {} behaviors {} }"
            )),
            WITH_TYPES
        );
    }

    // The same claim over a graph line, where there is more to normalize: the
    // §5 column alignment, an uppercase leading-zero handle, a leading-zero
    // depth, a trailing-zero float, and four OPTIONAL sets written `[]` — all
    // of them legal, none of them what the emitter writes.
    #[test]
    fn a_non_canonical_graph_source_normalizes() {
        let source = concat!(
            "module App { types {} graph { ",
            "scope App  exposes []  effects []  reset_on_reopen [] { ",
            "recurrent   App.total : i32  uses B@00D1  inputs []  depth 007  lookback [] ",
            " init 0.50  fallbacks []  members [] ",
            "} } behaviors {} }"
        );

        assert_eq!(
            emit(&parsed(source)),
            concat!(
                "module App {\n",
                "  types {}\n",
                "  graph {\n",
                "    scope App exposes [] effects [] {\n",
                "      recurrent App.total : i32 uses B@d1 depth 7 init 0.5\n",
                "    }\n",
                "  }\n",
                "  behaviors {}\n",
                "}\n"
            )
        );
    }

    // A `fallback_map` entry's two halves normalize the two DIFFERENT ways, and
    // one line shows both: the key is structural, so `003` is converted and
    // written back as the number three (R57), while the value is typed, stays
    // text, and is normalized TEXTUALLY where the literal is built (033-257) —
    // `007` to `7`, `-1.50` to `-1.5`, `00.0` to `0.0`.
    #[test]
    fn a_fallback_entrys_two_halves_normalize_by_their_own_rules() {
        let source = concat!(
            "module App { types {} graph { scope App exposes [] effects [] { ",
            "recurrent App.fib : f64 uses B@d8 depth 3 fallbacks [1: 007, 2: -1.50, 003: 00.0] ",
            "} } behaviors {} }"
        );

        assert_eq!(
            emit(&parsed(source)),
            concat!(
                "module App {\n",
                "  types {}\n",
                "  graph {\n",
                "    scope App exposes [] effects [] {\n",
                "      recurrent App.fib : f64 uses B@d8 depth 3 fallbacks [1: 7, 2: -1.5, 3: 0.0]\n",
                "    }\n",
                "  }\n",
                "  behaviors {}\n",
                "}\n"
            )
        );
    }

    // 033-268's by-content pick, which is the ONE canonicalization that reads a
    // cell's VALUES rather than its shape: a `fallbacks` map that seeds every
    // depth with one value MEANS `init v`, so `init v` is what it is written as.
    // M1.md §6.1's witness 11, third clause.
    #[test]
    fn an_equal_valued_fallback_map_canonicalizes_to_init() {
        let source = with_entry(
            "recurrent App.r : i64 uses B@d1 inputs [App.x] depth 2 fallbacks [1: 5, 2: 5]",
        );
        let canonical = with_entry("recurrent App.r : i64 uses B@d1 inputs [App.x] depth 2 init 5");

        assert_eq!(emit(&parsed(&source)), canonical);
        // And it is a FIXPOINT: `init 5` re-parses to the same meaning and comes
        // back out unchanged, so one meaning keeps exactly one spelling.
        assert_eq!(emit(&parsed(&canonical)), canonical);

        // An omitted `depth` is one slot (C-7), so a one-key map covers the whole
        // ring and folds exactly as the two-key one does.
        assert_eq!(
            emit(&parsed(&with_entry(
                "recurrent App.r : i64 uses B@d1 inputs [App.x] fallbacks [1: 5]"
            ))),
            with_entry("recurrent App.r : i64 uses B@d1 inputs [App.x] init 5")
        );

        // The literals are already textually canonical when they arrive
        // (033-257), so two spellings of one number are one value here.
        assert_eq!(
            emit(&parsed(&with_entry(
                "recurrent App.r : i64 uses B@d1 inputs [App.x] depth 2 fallbacks [1: 007, 2: 7]"
            ))),
            with_entry("recurrent App.r : i64 uses B@d1 inputs [App.x] depth 2 init 7")
        );
    }

    // The other side: every shape that is NOT one uniform seed renders exactly
    // as written. The general form is canonical when the values differ, and an
    // ill-formed map is the loader's to refuse — the emitter repairs nothing
    // (F4, R21).
    #[test]
    fn a_fallback_map_that_is_not_one_uniform_seed_renders_unchanged() {
        for line in [
            // The values differ: the general form IS the canonical spelling.
            "recurrent App.r : i64 uses B@d1 inputs [App.x] depth 2 fallbacks [1: 5, 2: 6]",
            // A hole — `missing_fallback_key` at load, and `init 5` would be a
            // different program.
            "recurrent App.r : i64 uses B@d1 inputs [App.x] depth 3 fallbacks [1: 5, 2: 5]",
            // A key outside the ring: `invalid_history_depth` at load.
            "recurrent App.r : i64 uses B@d1 inputs [App.x] depth 1 fallbacks [2: 5]",
            // One depth seeded twice: `duplicate_fallback_key` at load.
            "recurrent App.r : i64 uses B@d1 inputs [App.x] depth 2 fallbacks [1: 5, 1: 5]",
            // Not a recurrent — `fallbacks` names no history on any other kind.
            "input App.q : i64 fallbacks [1: 5]",
            // Both clauses: one meaning spelled twice, which is a LOAD error
            // (033-268) and therefore not a spelling to canonicalize.
            "recurrent App.r : i64 uses B@d1 inputs [App.x] depth 1 init 5 fallbacks [1: 5]",
        ] {
            let source = with_entry(line);

            assert_eq!(emit(&parsed(&source)), source, "{line}");
        }
    }

    // Fixpoint: emitting a parsed module, then parsing and emitting THAT, is
    // the same text. Anything the emitter writes that it cannot read back — a
    // missing space, a glued `->` — shows up here.
    #[test]
    fn emitted_text_parses_back_to_the_same_text() {
        for source in [CANONICAL, ONE_LINE, BLANK_LINES] {
            let once = emit(&parsed(source));
            let twice = emit(&parsed(&once));

            assert_eq!(once, CANONICAL);
            assert_eq!(twice, once);
        }

        for source in [WITH_TYPES, WITNESS, GRAPH_WITNESS] {
            let once = emit(&parsed(source));
            let twice = emit(&parsed(&once));

            assert_eq!(once, source);
            assert_eq!(twice, once);
        }

        // Every tag form, in one row, through the same two rounds.
        let rich = with_types(concat!(
            "%All = record { a: u128, b: %Request, c: pool_index<%Request>, ",
            "d: yielded<i32>, e: handle<portal<bool>>, f: (i8, [f32; 12]), ",
            "g: closure<(%Request, closure<() -> str>) -> (bool, str)> } size 0 align 1"
        ));

        assert_eq!(emit(&parsed(&rich)), rich);
        assert_eq!(emit(&parsed(&emit(&parsed(&rich)))), rich);
    }

    // The producer contract, executable. A name the lexer could never have
    // produced is a bug in whoever built the module, and the debug assert says
    // so instead of emitting text that does not lex. `debug_assert` compiles
    // out in release, so the cfg_attr keeps a `--release` run honest.
    #[test]
    #[should_panic(expected = "a module name is NAME-shaped")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn a_name_that_is_not_name_shaped_trips_the_producer_contract() {
        let mut module = parsed(CANONICAL);
        module.name = "not a name".to_string();

        let _ = emit(&module);
    }

    // The guard's OTHER conjunct, and the reason it is not redundant: `null` is
    // perfectly NAME-shaped — the charset cannot tell a reserved spelling from
    // an ordinary word — so a shape-only guard would have emitted
    // `module null { … }`, which the parser rejects as `reserved_word`. The
    // module name is the representative; the same conjunct rides at every other
    // NAME position.
    #[test]
    #[should_panic(expected = "a module name is NAME-shaped and never reserved")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn a_reserved_name_trips_the_producer_contract() {
        let mut module = parsed(CANONICAL);
        module.name = "null".to_string();

        let _ = emit(&module);
    }

    // The same contract at a NESTED name position. `Module::name` is the
    // shallowest of them; a type id inside a tag is the deepest thing the guard
    // reaches, and it is the representative for the whole name family.
    #[test]
    #[should_panic(expected = "a type id is NAME-shaped")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn a_nested_name_that_is_not_name_shaped_trips_the_producer_contract() {
        let mut module = parsed(&with_types("%A = record { v: %B } size 0 align 1"));

        let Layout::Record(fields) = &mut module.types.type_defs[0].layout else {
            panic!("the layout is a record");
        };

        fields[0].tag = TypeTag::TypeId("not a name".to_string());

        let _ = emit(&module);
    }

    // There is no third guard family here any more. A path spelled by one of
    // the five reserved words cannot reach the emitter at all: `CellPath::new`
    // rejects the segment when the path is BUILT (002-39), which is one layer
    // earlier and covers every path position rather than the two a keyword used
    // to shadow. `path::cell_path`'s `new_rejects_a_reserved_name` is that test.
    //
    // What this file still owes the reservation is the EMISSION half: a key
    // spelled by one of the five must stay quoted, or the text it writes would
    // read back as the keyword — a fixpoint-stable, meaning-changing
    // canonicalization (033-243).
    #[test]
    fn a_key_spelled_by_a_reserved_word_stays_quoted_and_round_trips() {
        for entry in [
            "wire App.w from App.'nan'.x to App.rows.'null'.render_row",
            "wire App.w from App.a to 'null'",
            "effect App.p reconciler \"r\" params [flag: App.'true', other: 'null'.x]",
            "fold App.s : i32 combiner B@1 else 0 members [App.'inf': permanent]",
        ] {
            let source = with_entry(entry);

            assert_eq!(emit(&parsed(&source)), source);
        }

        // And the bare/quoted decision itself, at the renderer.
        let keyed = CellPath::new(vec![Segment::Key("null".to_string())]);

        assert_eq!(keyed.render(), "'null'");
    }

    // The key half of `every_string_spelling_reads_back_as_its_own_value`, and
    // the two contents FND-21 was filed for, pinned as FIXED. Under the old
    // two-escape key set `{\', \\}` a carriage return had NO key spelling at
    // all: `Key("a\rb")` emitted a raw CR, which does not lex, and
    // `Key("a\r\nb")` emitted a raw CRLF, which intake normalizes — so the key
    // came back as `"a\nb"`, silently corrupted. Both spell `\r` now (R63).
    //
    // The proptest below draws key content from `any::<char>()` and covers
    // these in principle only; it will not draw THESE in practice, which is
    // why the defect's own cases are enumerated rather than left to sampling.
    #[test]
    fn a_key_spelling_reads_back_as_its_own_content() {
        for content in [
            "a\rb", "a\r\nb", "\r", "a\nb\tc", "a\u{0}b", "it's", "a\\b", "",
        ] {
            let keyed = CellPath::new(vec![
                Segment::Name {
                    text: "App".to_string(),
                    ordinal: None,
                },
                Segment::Key(content.to_string()),
                Segment::Name {
                    text: "row".to_string(),
                    ordinal: None,
                },
            ]);
            let source = with_entry(&format!("input {} : i32", keyed.render()));
            let module = parsed(&source);

            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            assert_eq!(
                cell.path.segments()[1],
                Segment::Key(content.to_string()),
                "{content:?}"
            );
            assert_eq!(emit(&module), source, "{content:?}");
        }
    }

    // The other family: an arity the grammar makes non-empty. An `enum` with no
    // variants is the representative — no parse produces one, so only a
    // hand-built module can be in that state.
    #[test]
    #[should_panic(expected = "at least one variant")]
    #[cfg_attr(not(debug_assertions), ignore = "debug_assert compiles out")]
    fn an_empty_arity_trips_the_producer_contract() {
        let mut module = parsed(&with_types("%A = record {} size 0 align 1"));
        module.types.type_defs[0].layout = Layout::Enum(Vec::new());

        let _ = emit(&module);
    }

    proptest! {
        // The unit tests above pin three hand-written spellings of the minimal
        // module. This asserts the same two laws over EVERY module name and
        // every legal whitespace filling: emitting a parsed module yields the
        // one canonical text, and emitting that again changes nothing.
        //
        // `gaps` fill the positions where a newline is legal; `spaces` fill the
        // positions where only spaces are (inside the header, and between a
        // section keyword and its brace) — the two halves of `parse`'s newline
        // law, generated rather than enumerated.
        #[test]
        fn every_legal_spelling_emits_the_one_canonical_text(
            name in "[A-Za-z_][A-Za-z0-9_]{0,19}",
            header_space in " {1,3}",
            gaps in prop::collection::vec(r"[ \n]{0,3}", 9),
            spaces in prop::collection::vec(" {0,3}", 4),
        ) {
            let source = [
                gaps[0].as_str(),
                "module",
                header_space.as_str(),
                name.as_str(),
                spaces[0].as_str(),
                "{",
                gaps[1].as_str(),
                "types",
                spaces[1].as_str(),
                "{",
                gaps[2].as_str(),
                "}",
                gaps[3].as_str(),
                "graph",
                spaces[2].as_str(),
                "{",
                gaps[4].as_str(),
                "}",
                gaps[5].as_str(),
                "behaviors",
                spaces[3].as_str(),
                "{",
                gaps[6].as_str(),
                "}",
                gaps[7].as_str(),
                "}",
                gaps[8].as_str(),
            ]
            .concat();

            let canonical =
                format!("module {name} {{\n  types {{}}\n  graph {{}}\n  behaviors {{}}\n}}\n");
            let once = emit(&parsed(&source));
            let twice = emit(&parsed(&once));

            prop_assert_eq!(&once, &canonical);
            prop_assert_eq!(&twice, &once);
        }

        // TOTALITY and UNIQUENESS over `str`, generated rather than enumerated
        // (R40): every value — quotes, backslashes, newlines, tabs, carriage
        // returns and control scalars included — has a spelling this emitter
        // writes, that spelling reads back AS the value, and emitting what it
        // read changes nothing. The unit test above pins the interesting
        // spellings; this asserts there are no others.
        #[test]
        fn every_str_value_has_one_canonical_spelling(
            content in prop::collection::vec(any::<char>(), 0..6)
                .prop_map(|characters| characters.into_iter().collect::<String>()),
        ) {
            let spelled = rendered(|out| string(out, &content));
            let source = with_entry(&format!("input App.x : str init {spelled}"));
            let module = parsed(&source);

            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            prop_assert!(matches!(&cell.init, Some(ValueLiteral::Str(text)) if text == &content));
            prop_assert_eq!(emit(&module), source);
        }

        // The same two laws over KEY content, which R63 made total: every
        // content — quotes, backslashes, newlines, tabs, carriage returns,
        // control scalars, and the five reserved spellings — has a spelling
        // this emitter writes, that spelling reads back AS the content, and
        // emitting what it read changes nothing.
        //
        // The generator draws from the same alphabet the `str` property does,
        // and the reserved spellings are drawn deliberately: `null` is
        // NAME-shaped, so it is the one content whose bare rendering would
        // round-trip to a DIFFERENT THING (033-243).
        //
        // Which VARIANT the content comes back in is not part of the claim. A
        // bare-rendered key is indistinguishable from a name segment in
        // isolation and parses as `Name`, which is 033-243's own reading —
        // "the graph structure decides which positions are keyed". What must
        // hold is that the CONTENT survives and the bytes are stable.
        #[test]
        fn every_key_content_has_one_canonical_spelling(
            content in prop_oneof![
                9 => prop::collection::vec(any::<char>(), 0..6)
                    .prop_map(|characters| characters.into_iter().collect::<String>()),
                1 => prop::sample::select(vec![
                    "null".to_string(),
                    "true".to_string(),
                    "false".to_string(),
                    "nan".to_string(),
                    "inf".to_string(),
                ]),
            ],
        ) {
            let keyed = CellPath::new(vec![
                Segment::Name { text: "App".to_string(), ordinal: None },
                Segment::Key(content.clone()),
                Segment::Name { text: "row".to_string(), ordinal: None },
            ]);
            let source = with_entry(&format!("input {} : i32", keyed.render()));
            let module = parsed(&source);

            let [Entry::Cell(cell)] = module.graph.scopes[0].entries.as_slice() else {
                panic!("the scope holds exactly one cell");
            };

            // TOTALITY: the content came back, whichever segment variant the
            // bare/quoted decision put it in.
            let read = match &cell.path.segments()[1] {
                Segment::Key(key) => key,
                Segment::Name { text, ordinal: None } => text,
                other => panic!("a middle segment carries no ordinal here: {other:?}"),
            };

            prop_assert_eq!(read, &content);
            // And the bare/quoted decision is the canonical one, so a key that
            // MUST quote did.
            prop_assert_eq!(
                matches!(&cell.path.segments()[1], Segment::Key(_)),
                !is_name_shaped(&content) || is_reserved_word(&content)
            );
            // UNIQUENESS: emitting what was read changes nothing.
            prop_assert_eq!(emit(&module), source);
        }
    }
}
