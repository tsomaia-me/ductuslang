use crate::compiler::Compiler;
use crate::selector::Selector;
use crate::syntax::front_end_goldens::{
    Corpus, fixture_corpus, generated_corpus, parser_test_corpus, robustness_quarter, source,
    spec_and_lesson_corpora,
};

#[path = "../../../tests/reviewed/mod.rs"]
mod reviewed;

use reviewed::{REVIEWED, reviewed};

fn spanned(text: &str, found: &[crate::diagnostics::Diagnostic]) -> Vec<(String, String)> {
    let mut spans: Vec<(String, String)> = found
        .iter()
        .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::NoEntryInstance)
        .map(|diagnostic| {
            let range = diagnostic.span.range.start as usize..diagnostic.span.range.end as usize;
            (
                diagnostic.code.to_string(),
                text.get(range).unwrap_or_default().to_string(),
            )
        })
        .collect();
    spans.sort();
    spans
}

#[test]
fn the_reviewed_moves_draw_their_expected_sets_on_the_new_core_464_476() {
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (ledger, text, _) in REVIEWED.iter().copied() {
        let found = new
            .checked_by_new_core(&[source(text)])
            .expect("the new core takes the program");
        let expected = reviewed(text).map(|(_, expected)| expected);
        assert_eq!(Some(spanned(text, &found)), expected, "{ledger}: {text}");
    }
}

fn assert_equal(corpora: &[Corpus]) -> u64 {
    let mut taken = 0;
    let mut differing = Vec::new();
    let mut moved = Vec::new();
    for corpus in corpora {
        let mut old = Compiler::new(&corpus.vocabulary).expect("the vocabulary compiles");
        old.select(Selector::OldEngine);
        let new = Compiler::new(&corpus.vocabulary).expect("the vocabulary compiles");
        for text in &corpus.texts {
            let sources = [source(text)];
            let Some(found) = new.checked_by_new_core(&sources) else {
                continue;
            };
            taken += 1;
            let expected = old.check(&sources);
            let without_reactive_lets: Vec<_> = found
                .iter()
                .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::ReactiveLet)
                .cloned()
                .collect();
            if let Some((ledger, wanted)) = reviewed(text) {
                if spanned(text, &found) == wanted {
                    moved.push(format!("{ledger} {}:\n{text}", corpus.name));
                } else {
                    differing.push(format!(
                        "{ledger} {}:\n{text}\n  new core {found:#?}",
                        corpus.name
                    ));
                }
                continue;
            }
            if found != expected && without_reactive_lets == expected {
                moved.push(format!("464 {}:\n{text}", corpus.name));
                continue;
            }
            let old_without_reactive_lets: Vec<_> = expected
                .iter()
                .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::ReactiveLet)
                .cloned()
                .collect();
            if found != expected
                && found == old_without_reactive_lets
                && faulty_line_let(text, &corpus.vocabulary)
            {
                moved.push(format!("455 {}:\n{text}", corpus.name));
                continue;
            }
            if found != expected {
                differing.push(format!(
                    "{}:\n{text}\n  new core {found:#?}\n  old engine {expected:#?}",
                    corpus.name
                ));
            }
        }
    }
    eprintln!("the new core took {taken} programs");
    eprintln!(
        "{} of them move by a reviewed ledger rule (455, 464, 476):\n{}",
        moved.len(),
        moved.join("\n")
    );
    let shown: Vec<&str> = differing.iter().take(5).map(String::as_str).collect();
    assert!(
        differing.is_empty(),
        "{} of {taken} programs the new core took differ from the old engine:\n{}",
        differing.len(),
        shown.join("\n")
    );
    assert!(taken > 0, "the new core took no program");
    taken
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_differential_the_spec_and_lesson_examples() {
    assert_equal(&spec_and_lesson_corpora());
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_differential_the_fixtures_and_the_parser_tests() {
    assert_equal(&[fixture_corpus(), parser_test_corpus(), generated_corpus()]);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_differential_the_robustness_corpus() {
    let corpora: Vec<Corpus> = (0..4).flat_map(robustness_quarter).collect();
    assert_equal(&corpora);
}

const FRONT_END_FAULTS: [(&str, &[(&str, &str)]); 6] = [
    ("Clip c:\n    Note/(1 + )\n", &[("expression-syntax", ")")]),
    (
        "state a(1[)\nClip c:\n    Note/(a)\n",
        &[("expression-syntax", ")")],
    ),
    (
        "let x = 1 & 2\nClip c:\n    Note/(x + 1)\n",
        &[("unexpected-character", "&")],
    ),
    (
        "let y = (1 +\nClip c:\n    Note/(y + 1)\n",
        &[("expression-syntax", "Clip")],
    ),
    (
        "Clip c:\n    Note/60\nstate     Note/62 when fill_on\n",
        &[("cell-syntax", "/"), ("duplicate-name", "Note")],
    ),
    (
        "File ::= Item*\nClip c:\n    Note/(Item)\n",
        &[("placement-syntax", "::"), ("unknown-kind", "File")],
    ),
];

#[test]
fn a_program_whose_front_end_drew_a_fault_compiles_through_the_new_core_by_the_placeholders() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, expected) in FRONT_END_FAULTS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const SHARED_STEP: [(&str, &[(&str, &str)]); 12] = [
    (
        "state level(0.0)\nlet l = level * 2state held = if l > 1 then l else held.past(1, 0.0)\nClip c | gain=held:\n    Note\n",
        &[("let-syntax", "held"), ("unknown-suffix", "state")],
    ),
    (
        "state level(0.0)\nlet l = level * 2\nClip c | gain=l:\n    Note\n",
        &[("reactive-let", "l")],
    ),
    (
        "Clip c:\n    Note/(j[^1])\nlet j = if k[0] > 1 then k else e + [3]\nlet e = []\n([1, 2]): i32[..4]\nstate k\n",
        &[("cell-syntax", "\n"), ("top-level-syntax", "(")],
    ),
    (
        "Clip c:\n    Note/(j[^1])\nlet j = if k[0] > 1 then k else e + [3]\nlet e = []\nstate k([1, 2])\n",
        &[("reactive-let", "j")],
    ),
    (
        "fn chord(big): if big then [60, 64, 67] else [60, 64]\n\nlet tones = chord(false)\n\nClip c:\n    Note/(tones[1])\n    Note/(chord(false)[2])\n",
        &[("index-out-of-range", "2")],
    ),
    (
        "fn chord(big): if big then [60, 64, 67] else [60, 64]\n\nlet tones = chord(true)\n\nClip c:\n    Note/(tones[1])\n    Note/(chord(true)[2])\n",
        &[],
    ),
    (
        "state x(1)\nfn f(v): v * 2\nClip c:\n    Note/([x, 1.5, 300000000000][2])\n",
        &[
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "[x, 1.5, 300000000000][2]"),
        ],
    ),
    (
        "state x(1)\nfn f(v): v * 2\nClip c:\n    Note/([x, 1, 3000][2])\n",
        &[],
    ),
    (
        "state x(1)\nClip c:\n    Note/([x, 1.5, 300000000000][2])\n    Note/([1.5, 300000000000][1])\n    Note/([1.5, 2][1])\n    Note/([x, 1.5, 2][2])\n    Note/(1.5 + 300000000000)\n    Note/([x, 300000000000][1])\n    Note/([x, 1.5][0])\n",
        &[
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "[x, 300000000000][1]"),
            ("type-mismatch", "1.5 + 300000000000"),
            ("type-mismatch", "[1.5, 2][1]"),
            ("type-mismatch", "[1.5, 300000000000][1]"),
            ("type-mismatch", "[x, 1.5, 2][2]"),
            ("type-mismatch", "[x, 1.5, 300000000000][2]"),
        ],
    ),
    (
        "state x(1)\nClip c:\n    Note/([x, 3, 2][2])\n    Note/([x, 30000][1])\n    Note/([x, 1][0])\n",
        &[],
    ),
    (
        "state x(1)\nClip c:\n    Note/([nothing, 300000000000][0])\n    Note/([x, 300000000000][0])\n    Note/([1, 300000000000][0])\n    Note/([nothing, 300000000000][1])\n    Note/([x, 300000000000][1])\n    Note/([nothing, 1.5, 2.5][^1])\n    Note/([x, 1.5, 2.5][^1])\n",
        &[
            ("does-not-fit", "[x, 300000000000][1]"),
            ("type-mismatch", "[x, 1.5, 2.5][^1]"),
            ("unknown-name", "nothing"),
            ("unknown-name", "nothing"),
            ("unknown-name", "nothing"),
        ],
    ),
    (
        "state x(1)\nClip c:\n    Note/([1, 300000000000][0])\n    Note/([x, 3][1])\n    Note/([x, 1, 2][^1])\n",
        &[],
    ),
];

#[test]
fn the_shared_steps_programs_draw_the_old_engines_sets_on_the_new_core_beside_their_valid_twins() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, expected) in SHARED_STEP {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const EXHAUSTIVE_CLASSES: [(&str, &[(&str, &str)]); 10] = [
    (
        "state f([1.5, 2.5])\nstate n(0)\nClip c:\n    Note/(f[n])\n",
        &[("type-mismatch", "f[n]")],
    ),
    (
        "state f([1.5, 2.5])\nClip c:\n    Note/(f[0])\n",
        &[("float-to-integer", "f[0]")],
    ),
    (
        "let q = i64(1)\nClip c:\n    Note/((i32(true) - 300000000000) % (1.5 / q))\n",
        &[
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "(i32(true) - 300000000000) % (1.5 / q)"),
        ],
    ),
    (
        "Clip c:\n    Note/((i32(1) - 300000000000) % 1.5)\n",
        &[
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "(i32(1) - 300000000000) % 1.5"),
        ],
    ),
    (
        "Clip c:\n    Note/((i32(1) - 300000000000) % i64(2))\n",
        &[
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "(i32(1) - 300000000000) % i64(2)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=((i32(1) - 300000000000) + 1.5)\n",
        &[("does-not-fit", "300000000000")],
    ),
    (
        "Clip c:\n    Note/(if (true + 1.5) then 1 else (if [1, 2][5] then 1 else 2))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("operand-type", "true + 1.5"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (1.5 is [1, 2]) then 1 else (if [1, 2][5] then 2 else 3))\n",
        &[
            ("array-operand", "1.5 is [1, 2]"),
            ("condition-type", "[1, 2][5]"),
        ],
    ),
    (
        "state q(false)\nClip c:\n    Note/(if (1 and q) then 1 else (if [1, 2][5] then 3 else 2))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/((if i32(true) then i32(true) else true)[300000000000 % i32(true)..])\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
];

#[test]
fn the_exhaustive_rows_classes_draw_the_old_engines_sets_on_the_new_core() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, expected) in EXHAUSTIVE_CLASSES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

fn nested_folds(depth: usize) -> String {
    let mut value = String::from("0");
    for level in (0..depth).rev() {
        value =
            format!("(for x{level} in [1] into a{level} from 0: a{level} + x{level} + {value})");
    }
    format!("let v = {value}\nClip c:\n    Note/([0][v])\n")
}

fn call_chain(length: usize) -> String {
    let mut text = String::new();
    for index in 0..length {
        text.push_str(&format!("fn step{index}(x): step{}(x)\n", index + 1));
    }
    text.push_str(&format!(
        "fn step{length}(x): x\nstate i(1)\nClip c:\n    Note/(step0(i))\n    Note/(step0(64))\n"
    ));
    text
}

#[test]
fn nested_folds_and_a_call_chain_type_and_evaluate_on_a_small_stack() {
    for text in [nested_folds(300), call_chain(1_000)] {
        let worker = std::thread::Builder::new()
            .stack_size(1 << 18)
            .spawn(move || {
                let compiler = Compiler::new(&symphonyscript_fixtures::vocabulary())
                    .expect("the vocabulary compiles");
                let sources = [source(&text)];
                let found = compiler
                    .checked_by_new_core(&sources)
                    .expect("the new core takes the program");
                assert!(
                    spanned(&text, &found)
                        .iter()
                        .all(|(code, _)| code == "index-out-of-range")
                );
            })
            .expect("the thread starts");
        worker
            .join()
            .expect("the work finishes without overflowing");
    }
}

const FAULTY_LINE_DECLARATIONS: [(&str, &str, bool); 6] = [
    ("let l = level * 2", "l", true),
    ("let l = level * 2 + 1.5", "l", true),
    ("let l = [level, 1][0]", "l", true),
    ("let l = f(level)", "l", true),
    ("state n = level * 2", "n", false),
    ("fn g(a): a * level", "g(1)", false),
];

const LINE_FAULTS: [&str; 4] = [" &", " )", " held = 1", "state"];

fn faulty_line_programs(fault: Option<&str>) -> Vec<(String, String)> {
    let mut programs = Vec::new();
    for (declaration, read, _) in FAULTY_LINE_DECLARATIONS {
        let line = match fault {
            Some(fault) => format!("{declaration}{fault}\n"),
            None => format!("{declaration}\n"),
        };
        let items = [
            "state level(0.0)\nfn f(v): v + 1\n".to_string(),
            line,
            format!("Clip c:\n    Note | velocity=({read})\n"),
        ];
        for order in [[0, 1, 2], [2, 1, 0]] {
            let text: String = order
                .iter()
                .filter_map(|at| items.get(*at).map(String::as_str))
                .collect();
            programs.push((text, declaration.to_string()));
        }
    }
    programs
}

#[test]
fn a_declaration_on_a_faulty_line_draws_no_reactive_let_and_otherwise_what_the_old_engine_draws_455()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut taken = 0;
    for fault in LINE_FAULTS {
        for (text, declaration) in faulty_line_programs(Some(fault)) {
            let sources = [source(&text)];
            let Some(found) = new.checked_by_new_core(&sources) else {
                continue;
            };
            taken += 1;
            let expected: Vec<_> = old
                .check(&sources)
                .into_iter()
                .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::ReactiveLet)
                .collect();
            if found != expected {
                failures.push(format!(
                    "{text}\n  new core {:?}\n  old engine without reactive-let {:?}",
                    spanned(&text, &found),
                    spanned(&text, &expected)
                ));
            }
            if declaration.starts_with("let") && fault == " &" && found.is_empty() {
                failures.push(format!("{text}\n  draws nothing, not even its fault"));
            }
        }
    }
    for (text, declaration) in faulty_line_programs(None) {
        let sources = [source(&text)];
        let Some(found) = new.checked_by_new_core(&sources) else {
            continue;
        };
        taken += 1;
        let reactive = FAULTY_LINE_DECLARATIONS
            .iter()
            .any(|(form, _, reactive)| *form == declaration && *reactive);
        let drawn: Vec<(String, String)> = spanned(&text, &found);
        let wanted: Vec<(String, String)> = if reactive {
            vec![("reactive-let".to_string(), "l".to_string())]
        } else {
            Vec::new()
        };
        if drawn != wanted || found != old.check(&sources) {
            failures.push(format!(
                "{text}\n  valid twin draws {drawn:?}, wanted {wanted:?}"
            ));
        }
    }
    assert!(taken >= 40, "the new core took {taken} programs");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn faulty_line_let(text: &str, vocabulary: &symphonyscript_core::Vocabulary) -> bool {
    let parsed = crate::syntax::parse(&source(text), vocabulary);
    let mut start = 0;
    text.split_inclusive('\n').any(|line| {
        let range = start + 1..start + line.trim_end_matches('\n').len();
        start += line.len();
        line.starts_with("let ")
            && parsed
                .diagnostics
                .iter()
                .any(|diagnostic| range.contains(&(diagnostic.span.range.start as usize)))
    })
}

const LOCAL_CLASHES: [(&str, &[(&str, &str)]); 10] = [
    (
        "fn f(Saw): Saw + 1\nClip c:\n    Note/(f(1))\n",
        &[("variant-name", "Saw")],
    ),
    (
        "fn f(Saw): Saw + true\nClip c:\n    Note/(f(1))\n",
        &[("operand-type", "Saw + true"), ("variant-name", "Saw")],
    ),
    (
        "fn f(x):\n    let Sine = x\n    Sine + 1\nClip c:\n    Note/(f(1))\n",
        &[("variant-name", "Sine")],
    ),
    (
        "let s = for Saw in 0..3 into a from 0: a + Saw\nClip c:\n    Note/(s)\n",
        &[("variant-name", "Saw")],
    ),
    (
        "let s = for i in 0..3 into a from 0:\n    let Saw = i\n    a + Saw\nClip c:\n    Note/(s)\n",
        &[("variant-name", "Saw")],
    ),
    (
        "let s = for Note in 0..3 into a from 0: a + Note\nClip c:\n    Note/(s)\n",
        &[("for-variable-clash", "Note")],
    ),
    (
        "let s = for abs in 0..3 into a from 0: a + abs\nClip c:\n    Note/(s)\n",
        &[("for-variable-clash", "abs")],
    ),
    (
        "let s = for i in 0..3 into a from 0:\n    let Note = i\n    a + Note\nClip c:\n    Note/(s)\n",
        &[("for-variable-clash", "Note")],
    ),
    (
        "let k = 1\nlet s = for i in 0..3 into a from 0:\n    let k = i\n    a + k\nClip c:\n    Note/(s)\n",
        &[("for-variable-clash", "k")],
    ),
    ("fn f(Note): Note + 1\nClip c:\n    Note/(f(1))\n", &[]),
];

#[test]
fn a_local_name_equal_to_a_variant_or_a_vocabulary_name_draws_as_the_old_engine_draws() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, expected) in LOCAL_CLASHES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
        assert_eq!(found, old.check(&sources), "{text}");
    }
}
