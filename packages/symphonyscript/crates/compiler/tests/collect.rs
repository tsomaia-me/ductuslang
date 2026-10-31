mod support;

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_core::{Count, Vocabulary};

fn with_prelude(prelude: &str) -> Vocabulary {
    let mut v = Vocabulary::builder("preluded");
    v.enumeration("Wave", ["Sine", "Saw"]);
    let clip = v.node("Clip");
    let note = v.node("Note");
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any);
    v.kind(note).attr::<i32>("pitch").default(60).default_attr();
    v.prelude("music.sym", prelude);
    v.build().expect("the vocabulary builds")
}

#[test]
fn a_top_level_name_declared_twice_is_an_error_naming_both() {
    let cases: &[(&str, u32, u32)] = &[
        ("state a(1)\nlet a = 2\n", 6, 15),
        ("let f = 1\nfn f(x): x\n", 4, 13),
        ("pattern P():\n    Note\nlet P = 1\n", 8, 26),
        ("Clip a:\n    Note\nlet a = 1\n", 5, 21),
        ("Clip a:\n    Note\nClip a:\n    Note\n", 5, 22),
    ];
    for (text, first, second) in cases {
        let diagnostics = support::checked(text);
        assert_eq!(diagnostics.len(), 1, "{text:?}: {diagnostics:?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.code, Code::DuplicateName, "{text:?}");
        assert_eq!(diagnostic.span.range, *second..second + 1, "{text:?}");
        assert_eq!(diagnostic.related.len(), 1, "{text:?}");
        assert_eq!(
            diagnostic.related[0].span.range,
            *first..first + 1,
            "{text:?}"
        );
    }
}

#[test]
fn files_meet_in_byte_order_of_their_paths() {
    let diagnostics = support::checked_files(&[("b.sym", "let x = 1\n"), ("a.sym", "let x = 2\n")]);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, Code::DuplicateName);
    assert_eq!(diagnostics[0].span.file, "b.sym");
    assert_eq!(diagnostics[0].related[0].span.file, "a.sym");
}

#[test]
fn diagnostics_come_by_file_in_top_level_order_with_the_prelude_first() {
    let vocabulary = with_prelude("fn half(x): x div 2\n");
    let compiler = Compiler::new(&vocabulary).expect("the prelude resolves");
    let source = Source {
        path: "a.sym".into(),
        text: "Clip c:\n    Note/(half(0.5))\n    Note/missing\n".into(),
    };
    let found: Vec<(String, Code)> = support::errors(compiler.check(&[source]))
        .into_iter()
        .map(|diagnostic| (diagnostic.span.file, diagnostic.code))
        .collect();
    assert_eq!(
        found,
        vec![
            ("music.sym".to_string(), Code::OperandType),
            ("a.sym".to_string(), Code::UnknownName),
        ]
    );
}

#[test]
fn a_source_name_equal_to_a_built_in_or_vocabulary_name_is_a_duplicate() {
    for name in ["sin", "pi", "rand", "Note", "Plays", "Wave"] {
        let text = format!("let {name} = 1\n");
        assert_eq!(
            support::check_codes(&text),
            vec![(Code::DuplicateName, 4, 4 + name.len() as u32)],
            "{name}"
        );
    }
}

#[test]
fn a_name_that_clashes_with_a_vocabulary_or_built_in_name_is_a_placeholder_at_every_reference() {
    let rows: &[support::Row] = &[
        (
            &["fn abs(x): x * 2\n", "Clip c:\n    Note/(abs(1.5))\n"],
            &[(Code::DuplicateName, "abs")],
        ),
        (
            &["let max = 5\n", "Clip c:\n    Note/max\n"],
            &[(Code::DuplicateName, "max")],
        ),
        (
            &[
                "Clip Note:\n    Note\n",
                "Clip c:\n    Note\n    Plays: Note\n",
            ],
            &[(Code::DuplicateName, "Note")],
        ),
        (
            &[
                "state Saw(1)\n",
                "Synth s:\n    Osc | wave=Saw\n",
                "Clip c:\n    Note/Saw\n",
            ],
            &[(Code::VariantName, "Saw")],
        ),
        (
            &[
                "Clip c:\n    Note/(i32(abs(-1.5)))\n    Plays: c\n",
                "Synth s:\n    Osc | wave=Saw\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_declared_name_equal_to_a_variant_is_an_error() {
    let text = "let Saw = 1\n";
    assert_eq!(support::check_codes(text), vec![(Code::VariantName, 4, 7)]);
}

#[test]
fn a_source_declaration_shadows_a_prelude_declaration() {
    let vocabulary = with_prelude("let base = 48\nfn twice(x): x * 2\n");
    let compiler = Compiler::new(&vocabulary).expect("the prelude resolves");
    let source = Source {
        path: "main.sym".into(),
        text: "let base = 60\nfn twice(x): x + x\n".into(),
    };
    assert_eq!(support::errors(compiler.check(&[source])), vec![]);
}

#[test]
fn a_prelude_that_declares_a_name_twice_or_holds_other_items_fails_the_compiler() {
    let cases: &[(&str, Code, &str)] = &[
        ("let a = 1\nlet a = 2\n", Code::DuplicateName, "a = 2"),
        ("let sin = 1\n", Code::DuplicateName, "sin"),
        ("state a(1)\n", Code::PreludeItem, "state a(1)\n"),
        (
            "Clip c:\n    Note\n",
            Code::PreludeItem,
            "Clip c:\n    Note\n",
        ),
    ];
    for (prelude, code, needle) in cases {
        let failed = match Compiler::new(&with_prelude(prelude)) {
            Ok(_) => panic!("{prelude:?} is rejected"),
            Err(failed) => failed,
        };
        assert_eq!(failed.diagnostics.len(), 1, "{prelude:?}");
        let diagnostic = &failed.diagnostics[0];
        assert_eq!(diagnostic.code, *code, "{prelude:?}");
        assert_eq!(diagnostic.span.file, "music.sym");
        let start = support::at(prelude, needle, 0).0;
        assert_eq!(diagnostic.span.range.start, start, "{prelude:?}");
    }
}

#[test]
fn a_pattern_that_reaches_itself_through_uses_is_an_error() {
    let direct = "pattern A():\n    Clip:\n        A\n";
    assert_eq!(
        support::check_codes(direct),
        vec![(Code::RecursivePattern, 8, 9)]
    );
    let mutual = "pattern A():\n    B\npattern B():\n    A\n";
    let diagnostics = support::checked(mutual);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, Code::RecursivePattern);
    assert_eq!(diagnostics[0].span.range, 8..9);
    assert_eq!(diagnostics[0].related[0].span.range, 27..28);
}
