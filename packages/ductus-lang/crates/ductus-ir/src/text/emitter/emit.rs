//! [`emit()`] — the canonical text emitter: the module AST back to `.duir`
//! (R54).

use std::fmt::{Arguments, Write};

use crate::module::{Module, TypesSection};
use crate::text::keywords::{
    ALIGN, BEHAVIORS, CLOSURE, ENUM, GRAPH, HANDLE, MODULE, POOL_INDEX, PORTAL, RECORD, SIZE,
    TUPLE, TYPES, YIELDED,
};
use crate::types::{Field, Layout, TypeDef, TypeTag, Variant};

/// One level of indentation: two spaces (R54).
const INDENT: &str = "  ";

/// The one list separator: comma, space. Written once so no caller can invent a
/// second spelling of it.
const SEPARATOR: &str = ", ";

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
///   and the id inside a `%NAME` or `pool_index<%NAME>` tag — is NAME-shaped;
/// - every ARITY the grammar makes non-empty — an `enum` layout's variants, a
///   `tuple` layout's elements, a tuple TAG's elements — is non-empty. (A
///   `record`'s field list and a `closure`'s parameter list are optional in the
///   grammar, so neither is asserted.)
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
            is_name_shaped(&module.name),
            "a module name is NAME-shaped: {}",
            module.name
        );

        self.block(format_args!("{MODULE} {}", module.name), |emitter| {
            emitter.types_section(&module.types);
            emitter.graph_section();
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
            is_name_shaped(&type_def.id),
            "a type id is NAME-shaped: {}",
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

    // graph_section ::= 'graph' '{' scope+ '}'
    fn graph_section(&mut self) {
        self.line(&format!("{GRAPH} {{}}"));
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
        is_name_shaped(&field.name),
        "a field name is NAME-shaped: {}",
        field.name
    );

    out.push_str(&field.name);
    out.push_str(": ");
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
        is_name_shaped(&variant.name),
        "a variant name is NAME-shaped: {}",
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
            debug_assert!(is_name_shaped(id), "a type id is NAME-shaped: {id}");

            out.push('%');
            out.push_str(id);
        }
        TypeTag::PoolIndex(id) => {
            debug_assert!(is_name_shaped(id), "a type id is NAME-shaped: {id}");

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

// The separator-writing loop the three list productions share: `SEPARATOR`
// before every element but the first, then the element's own rendering.
fn separated<T>(out: &mut String, items: &[T], render: impl Fn(&mut String, &T)) {
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push_str(SEPARATOR);
        }

        render(out, item);
    }
}

// NAME (§2): ASCII letters, digits and `_`, never digit-initial, never empty —
// the same shape `text::lexer::scan::ident` recognizes.
fn is_name_shaped(name: &str) -> bool {
    let mut characters = name.chars();

    let Some(first) = characters.next() else {
        return false;
    };

    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Span;
    use crate::module::{BehaviorsSection, GraphSection, TypesSection};
    use crate::text::test_support::{BLANK_LINES, CANONICAL, ONE_LINE, WITH_TYPES};
    use crate::text::test_support::{parsed, with_types};
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

    // The line-interior renderers write into a sink rather than returning one,
    // so a test that asserts on one construct's text supplies the sink.
    fn rendered(render: impl FnOnce(&mut String)) -> String {
        let mut out = String::new();
        render(&mut out);

        out
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

        for source in [WITH_TYPES, WITNESS] {
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
    }
}
