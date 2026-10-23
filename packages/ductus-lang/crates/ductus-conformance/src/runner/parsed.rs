//! The `parsed` function: a witness's claim 1, and the module claims 2–4 use.

use ductus_ir::module::Module;
use ductus_ir::text::emitter::emit;
use ductus_ir::text::lexer::lex;
use ductus_ir::text::parser::parse;

/// Lexes, parses and RE-EMITS `source`, asserting the emitted bytes are the
/// bytes that went in — then hands back the module.
///
/// That assertion is a witness's claim 1, and it is two claims in one. The
/// round trip is a round trip; and because the emitter IS the canonicalizer
/// (R54), byte equality also says the hand-written corpus file is already
/// canonical text. A corpus file that parsed but re-emitted differently would
/// be a file no tool would ever produce.
///
/// # Panics
///
/// If `source` does not lex, does not parse, or does not re-emit to itself —
/// each a broken corpus file rather than a runtime condition, so each fails
/// loudly and names the witness.
#[must_use]
pub fn parsed(witness: &str, source: &str) -> Module {
    let tokens = lex(source).expect("the witness lexes");
    let module = parse(tokens).expect("the witness parses");

    assert_eq!(
        emit(&module),
        source,
        "witness `{witness}`: the corpus file is not canonical text"
    );

    module
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "module App {\n  types {}\n  graph {}\n  behaviors {}\n}\n";

    #[test]
    fn hands_back_the_module_a_canonical_source_parses_to() {
        assert_eq!(parsed("empty", SOURCE).name, "App");
    }

    #[test]
    #[should_panic(expected = "is not canonical text")]
    fn refuses_a_source_that_does_not_re_emit_to_itself() {
        let uncanonical = SOURCE.replace("types {}", "types {\n  }");

        let _ = parsed("uncanonical", &uncanonical);
    }
}
