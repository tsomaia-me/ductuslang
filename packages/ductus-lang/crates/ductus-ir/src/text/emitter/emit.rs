//! [`emit()`] — the canonical text emitter: the module AST back to `.duir`
//! (R54).

use std::fmt::{Arguments, Write};

use crate::module::Module;

/// One level of indentation: two spaces (R54).
const INDENT: &str = "  ";

/// Render `module` as canonical `.duir` text (IR_GRAMMAR §3, §15.4.6).
///
/// The emitter IS the canonicalizer (R54): there is exactly one rendering of a
/// given module, so this function is also the definition of module equivalence
/// (R17) — two modules are the same when their emitted text is.
///
/// The canonical form of a module whose sections are empty:
///
/// ```text
/// module App {
///   types {}
///   graph {}
///   behaviors {}
/// }
/// ```
///
/// Two spaces per indent level, one construct per line, sections in grammar
/// order, an empty section as `name {}` with nothing between its braces, and
/// exactly one trailing newline.
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
/// case: `emit` `debug_assert`s the name's shape at the one place it writes it,
/// and otherwise trusts what it was given. Round-trip totality is a claim about
/// parsed modules, and it is stated here so the claim cannot be misread as one
/// about arbitrary values.
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

        self.block(format_args!("module {}", module.name), |emitter| {
            emitter.types_section();
            emitter.graph_section();
            emitter.behaviors_section();
        });
    }

    // types_section ::= 'types' '{' type_def* '}'
    //
    // No parameter: the section is empty in this increment, so nothing in the
    // AST reaches the output. `TypesSection` becomes an argument at increment
    // 5, when it carries `type_def`s to render.
    fn types_section(&mut self) {
        self.line("types {}");
    }

    // graph_section ::= 'graph' '{' scope+ '}'
    fn graph_section(&mut self) {
        self.line("graph {}");
    }

    // behaviors_section ::= 'behaviors' '{' behavior* '}'
    fn behaviors_section(&mut self) {
        self.line("behaviors {}");
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
    use crate::text::lexer::lex;
    use crate::text::parser::parse;
    use proptest::prelude::*;

    // Byte offsets counted by hand in `text::parser::parse`'s tests.
    const CANONICAL: &str = "module App {\n  types {}\n  graph {}\n  behaviors {}\n}\n";

    const ONE_LINE: &str = "module App { types {} graph {} behaviors {} }";

    const BLANK_LINES: &str =
        "\n\nmodule App {\n\n  types {}\n\n\n  graph {}\n\n  behaviors {}\n\n}\n\n";

    fn parsed(source: &str) -> Module {
        let tokens = lex(source).expect("source lexes");

        parse(tokens).expect("source parses")
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

    #[test]
    fn emitting_a_parsed_module_reproduces_its_canonical_source() {
        assert_eq!(emit(&parsed(CANONICAL)), CANONICAL);
    }

    // The emitter is the canonicalizer: a legal but non-canonical source
    // normalizes to the one true rendering.
    #[test]
    fn a_non_canonical_source_normalizes() {
        assert_eq!(emit(&parsed(ONE_LINE)), CANONICAL);
        assert_eq!(emit(&parsed(BLANK_LINES)), CANONICAL);
    }

    #[test]
    fn emitted_text_parses_back_to_the_same_text() {
        for source in [CANONICAL, ONE_LINE, BLANK_LINES] {
            let once = emit(&parsed(source));
            let twice = emit(&parsed(&once));

            assert_eq!(once, CANONICAL);
            assert_eq!(twice, once);
        }
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
