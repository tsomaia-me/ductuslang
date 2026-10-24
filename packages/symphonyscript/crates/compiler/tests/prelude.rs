mod support;

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_core::{Count, Vocabulary};

fn vocabulary(prelude: &str) -> Vocabulary {
    let mut v = Vocabulary::builder("preluded");
    let clip = v.node("Clip");
    let note = v.node("Note");
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any);
    v.kind(note)
        .attr::<i32>("pitch")
        .default(60)
        .default_attr()
        .attr::<f32>("velocity")
        .default(1.0);
    v.prelude("music.sym", prelude);
    v.build().expect("the vocabulary builds")
}

fn check(prelude: &str, program: &str) -> Vec<symphonyscript::Diagnostic> {
    let compiler = Compiler::new(&vocabulary(prelude)).expect("the prelude resolves");
    support::errors(compiler.check(&[Source {
        path: "main.sym".into(),
        text: program.into(),
    }]))
}

#[test]
fn a_program_uses_the_preludes_functions_patterns_and_values() {
    let prelude = "let C4 = 60\nfn up(n, by = 12): n + by\npattern Chord(default root):\n    Note/root\n    Note/(up(root, 4))\n";
    let program = "Clip c:\n    Chord/C4\n    Note/(up(C4))\n";
    assert_eq!(check(prelude, program), vec![]);
}

#[test]
fn a_prelude_body_sees_only_prelude_declarations() {
    let prelude = "let base = 60\nfn pitch(): base\n";
    let program = "let base = 3000000000\nClip c:\n    Note/(pitch())\n    Note/base\n";
    let found = check(prelude, program);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::DoesNotFit);
    assert_eq!(found[0].span.file, "main.sym");
}

#[test]
fn an_error_found_in_a_prelude_body_is_located_in_the_prelude() {
    let prelude = "fn loud(n): n * 2\n";
    let program = "Clip c:\n    Note/(loud(2000000000))\n";
    let found = check(prelude, program);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::DoesNotFit);
    assert_eq!(found[0].span.file, "music.sym");
    assert_eq!(found[0].span.range, 12..17);
    let prelude = "fn soft(n): n div 0.5\n";
    let program = "Clip c:\n    Note/(soft(2))\n";
    let found = check(prelude, program);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::OperandType);
    assert_eq!(found[0].span.file, "music.sym");
}
