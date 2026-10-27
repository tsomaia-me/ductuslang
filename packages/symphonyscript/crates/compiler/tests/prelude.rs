mod support;

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_core::{Count, Vocabulary};

fn vocabulary(prelude: &str) -> Vocabulary {
    vocabulary_at("music.sym", prelude)
}

fn vocabulary_at(path: &str, prelude: &str) -> Vocabulary {
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
    v.prelude(path, prelude);
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

#[test]
fn a_declaration_cycle_in_the_prelude_fails_the_compiler() {
    let rows: [&[&str]; 3] = [
        &["let a = b\n", "let b = a\n"],
        &["fn f(): a\n", "let a = f()\n"],
        &["fn up(n): n + 1\n", "let a = up(b)\n", "let b = up(a)\n"],
    ];
    for items in rows {
        for order in support::permutations(items) {
            let prelude = order.concat();
            let found = match Compiler::new(&vocabulary(&prelude)) {
                Ok(_) => Vec::new(),
                Err(failed) => failed.diagnostics,
            };
            let cycles: Vec<(Code, &str)> = found
                .iter()
                .map(|diagnostic| (diagnostic.code, diagnostic.span.file.as_str()))
                .collect();
            assert_eq!(
                cycles,
                vec![(Code::DeclarationCycle, "music.sym")],
                "{prelude}"
            );
        }
    }
    let prelude = "let C4 = 60\nfn up(n): n + C4\nlet unread = up(1) div 0.5\n";
    assert_eq!(check(prelude, "Clip c:\n    Note/72\n"), vec![]);
    let found = check(
        prelude,
        "let a = b + 1\nlet b = a + 1\nClip c:\n    Note/a\n",
    );
    let codes: Vec<Code> = found.iter().map(|diagnostic| diagnostic.code).collect();
    assert_eq!(codes, vec![Code::DeclarationCycle], "{found:?}");
}

#[test]
fn a_source_at_a_prelude_files_path_is_refused() {
    let compiler = Compiler::new(&vocabulary_at("x.sym", "fn f(x): x div 0.5\n"))
        .expect("the prelude resolves");
    let source = |path: &str, text: &str| Source {
        path: path.into(),
        text: text.into(),
    };
    let calls = "Clip c:\n    Note/(f(1))\n";
    let found = support::errors(compiler.check(&[source("x.sym", calls)]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::PreludePath);
    assert_eq!(found[0].span.file, "x.sym");
    assert!(found[0].summary.contains("`x.sym`"), "{found:?}");
    let found = support::errors(compiler.check(&[source("y.sym", calls)]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::OperandType);
    assert_eq!(found[0].span.file, "x.sym");
    let both = [
        source("x.sym", calls),
        source("y.sym", "Clip d:\n    Note/60\n"),
    ];
    for order in support::permutations(&[0, 1]) {
        let sources: Vec<Source> = order.iter().map(|index| both[*index].clone()).collect();
        let found = support::errors(compiler.check(&sources));
        let codes: Vec<(Code, &str)> = found
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.span.file.as_str()))
            .collect();
        assert_eq!(codes, vec![(Code::PreludePath, "x.sym")], "{found:?}");
    }
    let found = support::errors(compiler.check(&[source("y.sym", "Clip d:\n    Note/60\n")]));
    assert_eq!(found, vec![]);
}
