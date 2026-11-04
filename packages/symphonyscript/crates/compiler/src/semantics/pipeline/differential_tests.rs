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

const TWIN_PASS_TYPE_MISMATCHES: [&str; 10] = [
    "Clip c:\n    Note/((true is q)[if true then [1, 2][5] else [1, 2]])\n",
    "Clip c:\n    Note | velocity=((true is q)[if true then [1, 2][5] else [1, 2]])\n",
    "Clip c:\n    Note/((true < q)[i32(true)[[1, 2]]])\n",
    "Clip c:\n    Note | velocity=((true < q)[i32(true)[[1, 2]]])\n",
    "Clip c:\n    Note/((1.5 is q)[i32(true)[1]])\n",
    "Clip c:\n    Note | velocity=((1.5 is q)[i32(true)[1]])\n",
    "Clip c:\n    Note/(([1, 2][5] or q)[true[true]..])\n",
    "Clip c:\n    Note | velocity=(([1, 2][5] or q)[true[true]..])\n",
    "Clip c:\n    Note/(if (1 div i32(true)) then (if [1, 2] then q else q) else (q or [1, 2]))\n",
    "Clip c:\n    Note | velocity=(if (1 div i32(true)) then (if [1, 2] then q else q) else (q or [1, 2]))\n",
];

#[test]
fn the_twin_pass_alone_adds_the_type_mismatches_of_its_476_programs() {
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let programs = REVIEWED
        .iter()
        .map(|(_, text, _)| (*text, false))
        .chain(TWIN_PASS_TYPE_MISMATCHES.iter().map(|text| (*text, true)));
    let mut wrong = Vec::new();
    for (text, twin_pass) in programs {
        let sources = [source(text)];
        let alone = new
            .checked_by_new_core_without_twins(&sources)
            .expect("the new core takes the program");
        let alone = spanned(text, &alone);
        let whole = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        let added: Vec<(String, String)> = spanned(text, &whole)
            .into_iter()
            .filter(|wanted| wanted.0 == "type-mismatch" && !alone.contains(wanted))
            .collect();
        if added.is_empty() == twin_pass {
            wrong.push(format!("{text}\n  the twin pass adds {added:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
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
            if found == expected {
                continue;
            }
            let (old_drawn, new_drawn) = (drawn(&expected), drawn(&found));
            match reviewed::moved(text, &old_drawn, &new_drawn) {
                Ok(classes)
                    if !classes.is_empty()
                        && kept(&found, &new_drawn, &old_drawn)
                            == kept(&expected, &old_drawn, &new_drawn) =>
                {
                    let named: Vec<String> = classes.iter().map(|class| class.named()).collect();
                    moved.push(format!("{} {}:\n{text}", named.join("; "), corpus.name));
                }
                _ => differing.push(format!(
                    "{}:\n{text}\n  new core {found:#?}\n  old engine {expected:#?}",
                    corpus.name
                )),
            }
        }
    }
    eprintln!("the new core took {taken} programs");
    eprintln!(
        "{} of them move by a reviewed ledger rule (455, 464, 476, 478):\n{}",
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

const EXHAUSTIVE_CLASSES: [(&str, &[(&str, &str)]); 13] = [
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
    (
        "Clip c:\n    Note/([1, 2][(1 - 300000000000) / 2])\n",
        &[
            ("index-type", "(1 - 300000000000) / 2"),
            ("does-not-fit", "1 - 300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note/([1, 2][300000000000 / 2])\n",
        &[
            ("index-type", "300000000000 / 2"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note/([1, 2][i32(1) / 300000000000])\n",
        &[
            ("index-type", "i32(1) / 300000000000"),
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

fn drawn(found: &[crate::diagnostics::Diagnostic]) -> Vec<reviewed::Drawn> {
    found
        .iter()
        .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::NoEntryInstance)
        .map(|diagnostic| {
            (
                diagnostic.code.to_string(),
                diagnostic.span.range.start as usize,
                diagnostic.span.range.end as usize,
            )
        })
        .collect()
}

fn kept(
    found: &[crate::diagnostics::Diagnostic],
    own: &[reviewed::Drawn],
    other: &[reviewed::Drawn],
) -> Vec<crate::diagnostics::Diagnostic> {
    found
        .iter()
        .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::NoEntryInstance)
        .zip(own)
        .filter(|(_, at)| other.contains(at))
        .map(|(diagnostic, _)| diagnostic.clone())
        .collect()
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

const SCALAR_ANNOTATIONS: [&str; 20] = [
    "let a: i32 = true\nClip k:\n    Note/a\n",
    "let a: u8 = 300\nClip k:\n    Note/a\n",
    "let a: f32 = 1\nClip k:\n    Note/a\n",
    "let a: i32 = [1]\nClip k:\n    Note/a\n",
    "let a: i64 = 1\nClip k:\n    Note/a\n",
    "let a: u8 = 300\nClip k:\n    Note/1\n",
    "let a: i32 = 1.5\nClip k:\n    Note/a\n",
    "let a: f32 = 1.5\nClip k:\n    Note | velocity=a\n",
    "let a: u8 = 1 + 300\nClip k:\n    Note/a\n",
    "let a: bool = 1\nClip k:\n    Note/(if a then 1 else 2)\n",
    "let a: i32 = 300000000000\nClip k:\n    Note/a\n",
    "let b = 1.5\nlet a: i32 = b\nClip k:\n    Note/a\n",
    "let a: i32 = i64(1)\nClip k:\n    Note/a\n",
    "let a: u8 = 255\nClip k:\n    Note/(a + 1)\n",
    "let a: u8 = 255\nClip k:\n    Note/([1, 2][a])\n",
    "let a: f64 = 1.5\nClip k:\n    Note | velocity=a\n",
    "let a: i32 = true + 1\nClip k:\n    Note/a\n",
    "let a: i32 = [1][3]\nClip k:\n    Note/a\n",
    "let a: i32 = q + 1.5\nClip c:\n    Note/(a)\n",
    "let a: i32 = if true then q else 1.5\nClip c:\n    Note/(a)\n",
];

#[test]
fn a_scalar_annotation_on_a_top_level_let_draws_as_the_old_engine_draws() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in SCALAR_ANNOTATIONS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const REACTIVE_ANNOTATIONS: [(&str, &[(&str, &str)]); 14] = [
    (
        "state s0 = [1]\nlet x: i32 = s0\nClip c:\n    Note/(x)\n",
        &[
            ("reactive-let", "x"),
            ("type-mismatch", "let x: i32 = s0\n"),
        ],
    ),
    (
        "state s0(1)\nlet x: i32 = s0 + 1.5\nClip c:\n    Note/(x)\n",
        &[("reactive-let", "x"), ("type-mismatch", "s0 + 1.5")],
    ),
    (
        "state s0(true)\nlet x: i32 = s0\nClip c:\n    Note/(x)\n",
        &[("reactive-let", "x"), ("type-mismatch", "s0")],
    ),
    (
        "state s0(1)\nlet x: i32 = s0\nClip c:\n    Note/(x)\n",
        &[("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet x: u8 = s0 + 300\nClip c:\n    Note/(x)\n",
        &[("does-not-fit", "300"), ("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet x: u8 = s0 + 300\n",
        &[("reactive-let", "x")],
    ),
    (
        "state s0(300)\nlet x: u8 = s0\nClip c:\n    Note/(x)\n",
        &[("reactive-let", "x")],
    ),
    (
        "state s0(-100)\nlet x: u8 = s0 + 300\nClip c:\n    Note/(x)\n",
        &[("does-not-fit", "300"), ("reactive-let", "x")],
    ),
    (
        "state s0(-100)\nlet x: u8 = s0 + 100\nClip c:\n    Note/(x)\n",
        &[("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet x: u8 = s0 + 254\nClip c:\n    Note/(x)\n",
        &[("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet x: u8 = 300 * s0 + (200 + 100)\nClip c:\n    Note/(x)\n",
        &[
            ("does-not-fit", "200 + 100"),
            ("does-not-fit", "300"),
            ("reactive-let", "x"),
        ],
    ),
    (
        "state s0(1)\nlet k = 300\nlet x: u8 = s0 - k\nClip c:\n    Note/(x)\n",
        &[("does-not-fit", "k"), ("reactive-let", "x")],
    ),
    (
        "state s0(1)\nstate d = s0 + 1\nlet x: u8 = d - 300\nClip c:\n    Note/(x)\n",
        &[("does-not-fit", "300"), ("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet x: u8 = s0 + u8(1) * 300\nClip c:\n    Note/(x)\n",
        &[("does-not-fit", "300"), ("reactive-let", "x")],
    ),
];

#[test]
fn a_let_reading_a_cell_draws_reactive_let_beside_its_annotations_fault_and_its_operands_fit_464() {
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, expected) in REACTIVE_ANNOTATIONS {
        let found = new
            .checked_by_new_core(&[source(text)])
            .expect("the new core takes the program");
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
    }
}

const RUNTIME_OPERANDS: [&str; 7] = [
    "state s0(1): u8\nClip c:\n    Note/([1, 2][s0 + (200 + 100)])\n",
    "state s0(1): u8\nClip c:\n    Note/([1, 2][s0 + 200 * 2])\n",
    "state s0(1): u8\nClip c:\n    Note/([1, 2][s0 + [1, 300][1]])\n",
    "state s0(1): u8\nClip c:\n    Note/([1, 2][s0 + (if true then 300 else 1)])\n",
    "state s0(1): u8\nClip c:\n    Note/([1, 2][s0 + -1])\n",
    "state s0(1): u8\nlet k = 300\nClip c:\n    Note/([1, 2][s0 div k])\n",
    "state s0(1): u8\nClip c:\n    Note/([1, 2][s0 + u8(1) * 300])\n",
];

#[test]
fn a_compile_time_operand_of_a_runtime_operation_is_computed_exactly_and_fitted_at_its_span() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in RUNTIME_OPERANDS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const UNTAKEN_UNDER_A_KEPT_CONDITION: [&str; 4] = [
    "let a = i32(if i32(true) then [1, 2][5] else [1, 2][5])\nClip c:\n    Note/(a)\n",
    "let a = i32(if i32(true) then [1, 2][5] else [1, 2][5])\nClip c:\n    Note | velocity=(a)\n",
    "let a = u8(if i32(true) then [1, 2][5] else 2)\nClip c:\n    Note/(a)\n",
    "let a: i32 = i32(if i32(true) then [1, 2][5] else [1, 2][5])\nClip c:\n    Note/(a)\n",
];

#[test]
fn a_choice_whose_condition_keeps_a_type_other_than_bool_takes_no_arm_when_its_let_is_read() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in UNTAKEN_UNDER_A_KEPT_CONDITION {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const TWIN_PASS_TWINS: [&str; 12] = [
    "state q(0): i32\n",
    "let q = 1\n",
    "state q(0.5): f64\n",
    "let q = 0.5\n",
    "state q(false)\n",
    "let q = false\n",
    "let q = \"a\"\n",
    "state q([]): i32[..4]\n",
    "let q = [1]\n",
    "state q(Wave::Sine)\n",
    "let q = Wave::Sine\n",
    "let q = c\n",
];

fn main_file(found: &[crate::diagnostics::Diagnostic]) -> Vec<reviewed::Drawn> {
    let main: Vec<crate::diagnostics::Diagnostic> = found
        .iter()
        .filter(|diagnostic| diagnostic.span.file == "main.sym")
        .cloned()
        .collect();
    drawn(&main)
}

#[test]
fn every_twin_of_the_twin_pass_draws_an_annotated_lets_type_mismatch_the_twin_pass_misses_476() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in [
        "let a: i32 = if true then q else true\nClip c:\n    Note/(a)\n",
        "Clip c:\n    Note/(a)\nlet a: i32 = if true then q else true\n",
    ] {
        let choice = text.find("if true").expect("the choice");
        let at = (
            "type-mismatch".to_string(),
            choice,
            choice + "if true then q else true".len(),
        );
        let mut common: Option<Vec<reviewed::Drawn>> = None;
        for twin in TWIN_PASS_TWINS {
            let sources = [
                source(text),
                crate::source::Source {
                    path: "twin.sym".into(),
                    text: twin.to_string(),
                },
            ];
            let on_new = main_file(&new.check(&sources));
            assert!(on_new.contains(&at), "{twin}{text}{on_new:?}");
            let on_old = main_file(&old.check(&sources));
            assert_eq!(
                on_old.contains(&at),
                twin != "state q(false)\n",
                "{twin}{text}{on_old:?}"
            );
            common = Some(match common {
                None => on_new,
                Some(common) => common
                    .into_iter()
                    .filter(|kept| on_new.contains(kept))
                    .collect(),
            });
        }
        let found = drawn(
            &new.checked_by_new_core(&[source(text)])
                .expect("the new core takes the program"),
        );
        assert_eq!(found, drawn(&old.check(&[source(text)])), "{text}");
        let mut twins = common.unwrap_or_default();
        twins.extend(
            found
                .iter()
                .filter(|drawn| drawn.0 == "unknown-name")
                .cloned(),
        );
        let classes = reviewed::missed_by_the_twin_pass(text, &twins, &found)
            .unwrap_or_else(|unexplained| panic!("{text}{unexplained:?}"));
        assert_eq!(classes.len(), 1, "{text}");
    }
}

const CELL_ANNOTATIONS: [&str; 21] = [
    "state q(0): i32\nClip c:\n    Note/(q)\n",
    "state q(1.5): i32\nClip c:\n    Note/(q)\n",
    "state q([1]): i32\nClip c:\n    Note/(q)\n",
    "state q(true): i32\nClip c:\n    Note/(q)\n",
    "state q(300): u8\nClip c:\n    Note/(q)\n",
    "state q(1): f32\nClip c:\n    Note/(q)\n",
    "state q(0.5): f64\nClip c:\n    Note/(q)\n",
    "state q(1): bool\nClip c:\n    Note/(q)\n",
    "state q(false): bool\nClip c:\n    Note/(q)\n",
    "state q(300000000000): i32\nClip c:\n    Note/(q)\n",
    "state q(i64(1)): i32\nClip c:\n    Note/(q)\n",
    "state q(u8(1)): i32\nClip c:\n    Note/(q)\n",
    "state q(-1): u8\nClip c:\n    Note/(q)\n",
    "state q(x): i32\nClip c:\n    Note/(q)\n",
    "state q(1 + true): i32\nClip c:\n    Note/(q)\n",
    "state q([1, 2][5]): i32\nClip c:\n    Note/(q)\n",
    "state q(300): u8\n",
    "state q(0): u8\nClip c:\n    Note/([1, 2][q])\n",
    "state q(0): u8\nlet a = q + 1\nClip c:\n    Note/(a)\n",
    "state q(0): i32\nClip c:\n    Note | velocity=(q)\n",
    "state q(0): u8\nClip c:\n    Note/(q + 300)\n",
];

#[test]
fn a_cells_scalar_annotation_types_its_reads_and_checks_its_init_as_the_old_engine_does() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in CELL_ANNOTATIONS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const DERIVED_CELL_ANNOTATIONS: [&str; 20] = [
    "state s(1)\nstate q: i32 = s + 1\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: u8 = s + 300\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: i32 = s + 1.5\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: f64 = s + 1.5\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: bool = s + 1\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: bool = s > 1\nClip c:\n    Note/(if q then 1 else 2)\n",
    "state s(1)\nstate q: i64 = s + 1\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: i64 = s + 1\nClip c:\n    Note/(q + 1)\n",
    "state s(1)\nstate q: i32 = s + i64(1)\nClip c:\n    Note/(q)\n",
    "state s(1.5): f64\nstate q: i32 = s + 1\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: u8 = s\nClip c:\n    Note/([1, 2][q])\n",
    "state s(1)\nstate q: i32 = s + true\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: f32 = s * 2\nClip c:\n    Note | velocity=(q)\n",
    "state s(1)\nstate q: i32 = s + 1\nlet a = q + 1\nClip c:\n    Note/(a)\n",
    "state s(1)\nstate q: u8 = s + 1\nClip c:\n    Note/(q + (u8(255) + 4))\n",
    "state s(1)\nstate q: i32 = x + 1\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: i32 = 1 + 1\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: u8 = s * 2\nClip c:\n    Note | velocity=(q)\n",
    "state s(0.5)\nstate q: f32 = s\nClip c:\n    Note | velocity=(q)\n",
    "state s(1)\nstate q: i32 = s + 1\n",
];

#[test]
fn a_derived_cells_scalar_annotation_types_its_reads_and_checks_its_rule_as_the_old_engine_does() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in DERIVED_CELL_ANNOTATIONS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const ANNOTATED_ARRAY_RULES: [(&str, &[(&str, &str)]); 5] = [
    (
        "state s(1)\nstate q: i32 = [s]\nClip c:\n    Note/(q)\n",
        &[("type-mismatch", "[s]")],
    ),
    (
        "state s(1)\nstate q: i32 = [s]\n",
        &[("type-mismatch", "[s]")],
    ),
    (
        "state s(1)\nstate q: i32 = [s, s + 1]\nClip c:\n    Note/(q + 1)\n",
        &[("type-mismatch", "[s, s + 1]")],
    ),
    (
        "state s(1)\nstate q: u8 = [s + 300]\nClip c:\n    Note/(q)\n",
        &[("type-mismatch", "[s + 300]")],
    ),
    (
        "state s(1)\nstate q: bool = [s > 1]\nClip c:\n    Note/(if q then 1 else 2)\n",
        &[("type-mismatch", "[s > 1]")],
    ),
];

const ANNOTATED_ARRAY_RULE_TWINS: [&str; 3] = [
    "state s(1)\nstate q: i32 = [s][0]\nClip c:\n    Note/(q)\n",
    "state s(1)\nstate q: i32 = s\nClip c:\n    Note/(q)\n",
    "state q([1]): i32\nClip c:\n    Note/(q)\n",
];

#[test]
fn an_annotated_derived_cells_array_rule_draws_its_type_mismatch_at_the_rule_476() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, expected) in ANNOTATED_ARRAY_RULES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
        assert!(
            found
                .iter()
                .filter(|diagnostic| diagnostic.code == crate::diagnostics::Code::TypeMismatch)
                .all(|diagnostic| diagnostic.summary
                    == "the rule's shape differs from the cell's type"),
            "{text}"
        );
        let before = old.check(&sources);
        let classes = reviewed::moved(text, &drawn(&before), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "476"),
            "{text}"
        );
    }
    for text in ANNOTATED_ARRAY_RULE_TWINS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const FLOWING_TYPES_AND_FLOAT_CONVERSIONS: [&str; 20] = [
    "state q(0): i32\nClip c:\n    Note/(q + (u8(255) + 4))\n",
    "state q(0): i32\nClip c:\n    Note/([1, 2][q + (u8(255) + 4)])\n",
    "state q(0)\nClip c:\n    Note/([1, 2][q + (u8(255) + 4)])\n",
    "state q(0)\nClip c:\n    Note/(q + (u8(255) + 4))\n",
    "state q(0): i32\nClip c:\n    Note/(if q > 1 then q + (u8(255) + 4) else 1)\n",
    "state q(0): i32\nClip c:\n    Note/([1, 2][if true then q + (u8(255) + 4) else 1])\n",
    "state q(0): i32\nClip c:\n    Note/(q * (q + (u8(255) + 4)))\n",
    "state q(0): i32\nfn f(x): x + (u8(255) + 4)\nClip c:\n    Note/(f(q))\n",
    "state q(0): i32\nClip c:\n    Note/(i32(q + (u8(255) + 4)))\n",
    "state q(0): i32\nClip c:\n    Note/(q > u8(255) + 4)\n",
    "state s(1)\nstate q: i32 = s + 1\nClip c:\n    Note/(q + (u8(255) + 4))\n",
    "state s(1)\nClip c:\n    Note/(f32(s))\n",
    "state s(1)\nClip c:\n    Note | velocity=(f32(s))\n",
    "state s(1)\nClip c:\n    Note/(f64(s))\n",
    "state s(1)\nClip c:\n    Note/(f32(s + 1))\n",
    "state s(1): i32\nClip c:\n    Note/(f32(s))\n",
    "state s(1): i32\nClip c:\n    Note/(f64(s))\n",
    "state s(1)\nClip c:\n    Note/(i32(f32(s)))\n",
    "state s(1)\nClip c:\n    Note/(f32(s + [1, 2][5]))\n",
    "state s(1.5)\nClip c:\n    Note/(f32(s))\n",
];

#[test]
fn a_type_flowing_in_equal_to_a_cells_own_computes_a_fixed_operand_in_it_and_a_refused_float_conversion_reads_its_cell_as_a_float()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in FLOWING_TYPES_AND_FLOAT_CONVERSIONS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const FLOAT_CELLS_THROUGH_A_CALL: [&str; 8] = [
    "state tw(0.5): f64\nfn h(z):\n    let x = tw\n    x\nClip c:\n    Note/(h(0))\n",
    "state tw(0.5)\nfn h(z):\n    let x = tw\n    x\nClip c:\n    Note/(h(0))\n",
    "state tw(0.5)\nfn h(z): tw\nClip c:\n    Note/(h(0))\n",
    "state tw(0.5)\nfn h(z):\n    let x = tw + 1\n    x\nClip c:\n    Note/(h(0))\n",
    "state s1 = [1.5]\nstate tw(0.5): f64\nfn pass(v): v\nClip c:\n    Note/((pass(s1))[0])\n",
    "state s1(1.5)\nfn pass(v): v\nClip c:\n    Note/(pass(s1))\n",
    "state s1(1.5)\nfn pass(v, w): w\nClip c:\n    Note/(pass(1, pass(2, s1)))\n",
    "state n1(false)\nfn pass(v): v\nClip c:\n    Note/((pass(if n1 then [1.5] else [1.5]))[0])\n",
];

#[test]
fn a_float_cell_read_through_a_call_or_a_local_let_where_an_integer_is_expected_draws_float_to_integer()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in FLOAT_CELLS_THROUGH_A_CALL {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const CELL_READS_IN_THE_USES_TYPE: [&str; 16] = [
    "state q(0): i64\nClip c:\n    Note/(q)\n",
    "state q(0): i64\nClip c:\n    Note/(q + 1)\n",
    "state q(0): i64\nClip c:\n    Note | velocity=(q)\n",
    "state q(0): i32\nClip c:\n    Note/(q)\n",
    "state s(1)\nClip c:\n    Note/(s + i64(1))\n",
    "state s(1)\nClip c:\n    Note/(s + i32(1))\n",
    "state s(1): i64\nClip c:\n    Note | velocity=(s + i64(1))\n",
    "state s(1)\nClip c:\n    Note/(s * (i64(1) + 2) + i64(3))\n",
    "state s(1)\nClip c:\n    Note/(if true then s + i64(1) else 2)\n",
    "state fl(0.5): f64\nClip c:\n    Note | velocity=(fl + f64(1))\n",
    "state s0(-100)\nClip c:\n    Note/([1, 2][s0 + u8(1)])\n",
    "state s0(-100)\nClip c:\n    Note/([1, 2][s0 + i32(1)])\n",
    "state s0(1)\nClip c:\n    Note/([1, 2][s0 + 10000000000000000000])\n",
    "state s0(1): u8\nClip c:\n    Note/(i64(s0 + 300))\n",
    "state s0(1): u8\nClip c:\n    Note/(i64(s0 + 100))\n",
    "state s(1)\nClip c:\n    Note | velocity=(f32(s))\n",
];

#[test]
fn a_cell_read_takes_the_type_its_use_expects_and_a_fixed_operand_beside_it_is_refused_at_itself() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in CELL_READS_IN_THE_USES_TYPE {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const TYPES_THROUGH_A_REACTIVE_LET: [(&str, &[(&str, &str)]); 8] = [
    (
        "state s0(1)\nlet x: u8 = s0 div 300\nClip c:\n    Note/(x)\n",
        &[("does-not-fit", "300"), ("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet x: u8 = s0 div 3\nClip c:\n    Note/(x)\n",
        &[("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet w = s0\nlet x: u8 = w + 300\nClip c:\n    Note/(x)\n",
        &[
            ("does-not-fit", "300"),
            ("reactive-let", "w"),
            ("reactive-let", "x"),
        ],
    ),
    (
        "state s0(1)\nlet w = s0\nlet x: u8 = w + 100\nClip c:\n    Note/(x)\n",
        &[("reactive-let", "w"), ("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet w = s0\nlet x: u8 = w + 300\n",
        &[("reactive-let", "w"), ("reactive-let", "x")],
    ),
    (
        "state s0(1)\nlet w = s0 + i64(1)\nClip c:\n    Note/(w)\n",
        &[("reactive-let", "w"), ("type-mismatch", "w")],
    ),
    (
        "state s0(1)\nlet w = s0\nClip c:\n    Note/(w + true)\n",
        &[("operand-type", "w + true"), ("reactive-let", "w")],
    ),
    (
        "state s0(1)\nlet w = [s0, 1]\nClip c:\n    Note/(w)\n",
        &[("reactive-let", "w"), ("type-mismatch", "w")],
    ),
];

#[test]
fn a_read_of_a_let_reading_a_cell_keeps_the_lets_known_type_and_its_operands_fit_464() {
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, expected) in TYPES_THROUGH_A_REACTIVE_LET {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
    }
}

const GATE_REPAIRS: [(&str, &str, &str); 2] = [
    (
        "\nstate base_pitch(48.0)\nlet pitch_offset = base_pitch * 0.5\n\nfn fifth(p):\n    p + 7\n\nClip melody:\n    Note/(base_pitch + 7)\n    Note/(fifth(base_pitch))\n    Note/(base_pitch + pitch_offset)\n\nSynth lead:\n    Osc | freq=(base_pitch * 2.5)\n",
        "\nstate base_pitch(48.0)\nstate pitch_offset = base_pitch * 0.5\n\nfn fifth(p):\n    p + 7\n\nClip melody:\n    Note/(base_pitch + 7)\n    Note/(fifth(base_pitch))\n    Note/(base_pitch + pitch_offset)\n\nSynth lead:\n    Osc | freq=(base_pitch * 2.5)\n",
        "pitch_offset",
    ),
    (
        "\nstate a(2)\nstate b(3)\nstate i(0)\n\nlet c = [b, a, b + a]\nlet mutated = [b * 2, a + 1, c[i] + 1]\n\nClip main:\n    Note/1 | velocity=a\n    Note/2 | velocity=b\n    Note/3 | velocity=c[i]\n",
        "\nstate a(2)\nstate b(3)\nstate i(0)\n\nstate c = [b, a, b + a]\nstate mutated = [b * 2, a + 1, c[i] + 1]\n\nClip main:\n    Note/1 | velocity=a\n    Note/2 | velocity=b\n    Note/3 | velocity=c[i]\n",
        "c mutated",
    ),
];

#[test]
fn a_gate_program_reading_a_reactive_let_draws_its_repaired_twins_set_beside_reactive_let_464() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, repaired, lets) in GATE_REPAIRS {
        let found = new
            .checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut wanted = spanned(repaired, &old.check(&[source(repaired)]));
        wanted.extend(
            lets.split(' ')
                .map(|name| ("reactive-let".to_string(), name.to_string())),
        );
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
    }
}

const CONVERSION_OPERANDS: [(&str, &[(&str, &str)]); 8] = [
    (
        "Clip c:\n    Note | velocity=(f32(u8(255) + 4))\n",
        &[("constant-overflow", "u8(255) + 4")],
    ),
    (
        "Clip c:\n    Note | velocity=(f64(u8(255) + 4))\n",
        &[
            ("constant-overflow", "u8(255) + 4"),
            ("type-mismatch", "f64(u8(255) + 4)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(f32(u8(1) + 300))\n",
        &[("does-not-fit", "300")],
    ),
    (
        "Clip c:\n    Note/(f32(u8(255) + 4))\n",
        &[
            ("constant-overflow", "u8(255) + 4"),
            ("type-mismatch", "f32(u8(255) + 4)"),
        ],
    ),
    (
        "state s(1)\nClip c:\n    Note | velocity=(f32(s + u8(1)))\n",
        &[],
    ),
    (
        "state s(1)\nClip c:\n    Note | velocity=(f32(s + i64(1)))\n",
        &[],
    ),
    (
        "state q(0): i64\nClip c:\n    Note | velocity=(f32(q + (u8(255) + 4)))\n",
        &[("constant-overflow", "u8(255) + 4")],
    ),
    (
        "state q(0)\nClip c:\n    Note | velocity=(f32(q + (u8(250) + 4)))\n",
        &[],
    ),
];

const CONVERSION_OPERAND_TWINS: [&str; 5] = [
    "Clip c:\n    Note | velocity=(f32(u8(250) + 4))\n",
    "state s(1)\nClip c:\n    Note | velocity=(f32(s + 1))\n",
    "Clip c:\n    Note/(i32(u8(255) + 4))\n",
    "Clip c:\n    Note/(i32(2.7))\n",
    "Clip c:\n    Note/(i32(2.7 + 1))\n",
];

#[test]
fn a_conversions_operand_is_computed_in_its_own_type_where_the_old_engine_flows_the_target_478() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (text, expected) in CONVERSION_OPERANDS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
        let before = old.check(&sources);
        assert_ne!(before, found, "{text}");
        let classes = reviewed::moved(text, &drawn(&before), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "478"),
            "{text}"
        );
    }
    for text in CONVERSION_OPERAND_TWINS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const FOLD_ANNOTATIONS: [&str; 24] = [
    "let n = for i in 0..3 into a: i32 from 0: a + i\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: u8 from 0: a + i + 300\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: u8 from 300: a + i\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: f32 from 0: a + i\nClip c:\n    Note | velocity=(n)\n",
    "let n = for i in 0..3 into a: bool from false: a or i > 1\nClip c:\n    Note/(if n then 1 else 2)\n",
    "let n = for i in 0..3 into a: i64 from 0: a + i\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: u8 from 0: a + 100\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: u8 from 250: a + 1\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: f64 from 0: a + i\nClip c:\n    Note | velocity=(n)\n",
    "let n = for i: i32 in 0..3 into a from 0: a + i\nClip c:\n    Note/(n)\n",
    "let n = for x: u8 in [1, 300] into a from 0: a + x\nClip c:\n    Note/(n)\n",
    "let n = for x: u8 in [1, 2] into a from 0: a + x\nClip c:\n    Note/(n)\n",
    "let n = for i: u8 in 0..300 into a from 0: a + i\nClip c:\n    Note/(n)\n",
    "let n = for x: i32 in [1.5] into a from 0: a + x\nClip c:\n    Note/(n)\n",
    "let n = for x: f32 in [1, 2] into a from 0: a + x\nClip c:\n    Note/(n)\n",
    "let n = for i, x: u8 in [1, 2] into a from 0: a + x + i\nClip c:\n    Note/(n)\n",
    "let n = for x: bool in [true] into a from 0: a + x\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: u8 from 0: 300\nClip c:\n    Note/(n)\n",
    "let n = for i: f32 in 0..3 into a from 0.0: a + i\nClip c:\n    Note | velocity=(n)\n",
    "let n = for x: f64 in [1, 2] into a from 0.0: a + x\nClip c:\n    Note | velocity=(n)\n",
    "fn f(k): for i in 0..k into a: u8 from 0: a + 100\nClip c:\n    Note/(f(3))\n",
    "let n = for i, x: i32 in [1.5] into a from 0: a + i\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: u8 from q: a + i\nClip c:\n    Note/(n)\n",
    "let n = for i in 0..3 into a: u8 from 0: a + (for j in 0..2 into b: u8 from 0: b + 200)\nClip c:\n    Note/(n)\n",
];

const FOLD_ANNOTATION_FAULTS: [(&str, &[(&str, &str)]); 6] = [
    (
        "let n = for i in 0..3 into a: i32 from 0.5: a + i\nClip c:\n    Note/(n)\n",
        &[("type-mismatch", "0.5")],
    ),
    (
        "let n = for i in 0..3 into a: i32 from [0]: a + [i]\nClip c:\n    Note/(n)\n",
        &[("array-operand", "a + [i]"), ("type-mismatch", "[0]")],
    ),
    (
        "let n = for i in 0..3 into a: i32 from 0: a + 1.5\nClip c:\n    Note/(n)\n",
        &[("type-mismatch", "a + 1.5")],
    ),
    (
        "let n = for i in 0..3 into a: u8 from 0: a + i64(1)\nClip c:\n    Note/(n)\n",
        &[("type-mismatch", "a + i64(1)")],
    ),
    (
        "let n = for x: i32 in [[1]] into a from 0: a + x\nClip c:\n    Note/(n)\n",
        &[("type-mismatch", "x")],
    ),
    (
        "let n = for i in 0..3 into a: bool from 0: a\nClip c:\n    Note/(if n then 1 else 2)\n",
        &[("type-mismatch", "0")],
    ),
];

#[test]
fn an_annotated_accumulator_or_loop_name_fixes_its_type_as_spec_4_5_states_476() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in FOLD_ANNOTATIONS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
    for (text, expected) in FOLD_ANNOTATION_FAULTS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
        let before = old.check(&sources);
        let classes = reviewed::moved(text, &drawn(&before), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "476"),
            "{text}"
        );
    }
}

const ARRAY_ANNOTATIONS: [&str; 24] = [
    "let x: i32[3] = [1, 2, 3]\nClip c:\n    Note/(x[1])\n",
    "let x: u8[2] = [1, 300]\nClip c:\n    Note/(x[1])\n",
    "let x: i32[3] = [1, 2]\nClip c:\n    Note/(x[1])\n",
    "let x: i32[2] = 1\nClip c:\n    Note/(x[1])\n",
    "let x: i32[2] = [1.5, 2]\nClip c:\n    Note/(x[1])\n",
    "let x: i32[3][2] = [[1, 2, 3], [4, 5, 6]]\nClip c:\n    Note/(x[1][2])\n",
    "let x: i32[2][3] = [[1, 2, 3], [4, 5, 6]]\nClip c:\n    Note/(x[1][2])\n",
    "let x: f32[2] = [1, 2]\nClip c:\n    Note | velocity=(x[1])\n",
    "let x: bool[2] = [true, false]\nClip c:\n    Note/(if x[0] then 1 else 2)\n",
    "let x: i32[2] = [1, 2]\nClip c:\n    Note/(x)\n",
    "let x: i32[2] = [1, 2] + [3]\nClip c:\n    Note/(x[1])\n",
    "state s(1)\nlet x: i32[2] = [s, 1]\nClip c:\n    Note/(x[1])\n",
    "let x: u8[2] = [1, 300]\n",
    "let x: u8[2] = [1, 2]\nlet y = x[0] + 300\nClip c:\n    Note/(y)\n",
    "let x: i32[2] = [1, 2]\nlet y = x + [3]\nClip c:\n    Note/(y[2])\n",
    "let x: i32[2] = [1, [2]]\nClip c:\n    Note/(x[0])\n",
    "let x: i32[2] = [true, 2]\nClip c:\n    Note/(x[0])\n",
    "let y = [1.5, 2]\nlet x: i32[2] = y\nClip c:\n    Note/(x[0])\n",
    "let x: f32[2] = [1, 2.5]\nClip c:\n    Note | velocity=(x[0])\n",
    "let x: i32[2] = [1, 2]\nClip c:\n    Note/(x[^1])\n",
    "let x: i32[3] = [1, 2, 3]\nClip c:\n    Note/(x[0..2][1])\n",
    "let x: u8[2] = [1, 2][0..2]\nClip c:\n    Note/(x[0])\n",
    "let x: i32[2] = [q, 1]\nClip c:\n    Note/(x[0])\n",
    "let x: i64 = 1\nClip c:\n    Note/(x)\n",
];

const ARRAY_ANNOTATION_FAULTS: [(&str, &[(&str, &str)]); 5] = [
    (
        "let x: u8[2] = [1, 300]\nClip c:\n    Note/(x[0])\n",
        &[("does-not-fit", "300")],
    ),
    (
        "let x: i64[2] = [1, 2]\nClip c:\n    Note/(x[1])\n",
        &[("type-mismatch", "x[1]")],
    ),
    (
        "let x: i64[2] = [1, 2]\nClip c:\n    Note | velocity=(x[1])\n",
        &[("type-mismatch", "x[1]")],
    ),
    (
        "let x: i32[2] = [1.5, 2.5]\nClip c:\n    Note/(x[0])\n",
        &[("type-mismatch", "1.5"), ("type-mismatch", "2.5")],
    ),
    (
        "let k = 300\nlet x: u8[2] = [1, k]\nClip c:\n    Note/(x[0])\n",
        &[("does-not-fit", "k")],
    ),
];

#[test]
fn an_array_annotation_on_a_top_level_let_fixes_its_whole_values_type_476() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in ARRAY_ANNOTATIONS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
    for (text, expected) in ARRAY_ANNOTATION_FAULTS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut wanted: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        assert_eq!(spanned(text, &found), wanted, "{text}");
        let before = old.check(&sources);
        let classes = reviewed::moved(text, &drawn(&before), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "476"),
            "{text}"
        );
    }
    for text in [
        "let x: i32[0] = []\nClip c:\n    Note/(1)\n",
        "state q([1, 2]): i32[2]\nClip c:\n    Note/(q[1])\n",
        "let n = for x: i32[2] in [[1, 2], [3, 4]] into a from 0: a + x[1]\nClip c:\n    Note/(n)\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}

const NATIVES_ARRAY_ATTRIBUTES: [&str; 17] = [
    "Clip c | trio=[1, 2, 3]\n",
    "Clip c | trio=[1, 2]\n",
    "Clip c | trio=1\n",
    "Clip c | trio=[1.5, 2, 3]\n",
    "Clip c | trio=[1, 2, 3000000000]\n",
    "Clip c | trio=[true, false, true]\n",
    "Clip c | trio=[[1], [2], [3]]\n",
    "let t = [1, 2, 3]\nClip c | trio=t + [4]\n",
    "state s(1)\nClip c | trio=[s, 2, 3]\n",
    "state s(1.5)\nClip c | trio=[s, 2, 3]\n",
    "Clip c | trio=[u8(1), 2, 3]\n",
    "Clip c | trio=[i64(1), 2, 3]\n",
    "Clip c | trio=[q, 2, 3]\n",
    "Clip c | trio=q\n",
    "Clip c | trio=[1, 2, 3][0]\n",
    "Clip c | gain=[1.0]\n",
    "Clip c | trio=[1 + true, 2, 3]\n",
];

const SIGNATURES_ARRAY_ATTRIBUTES: [&str; 11] = [
    "Clip c | fixed=[1, 2, 3]\n",
    "Clip c | fixed=[1, 2, 300]\n",
    "Clip c | fixed=[1, 2, -1]\n",
    "Clip c | fixed=[i32(1), 2, 3]\n",
    "Clip c | fixed=[u8(1), 2, 3]\n",
    "Clip c | fixed=[1.5, 2, 3]\n",
    "state s(1)\nClip c | fixed=[s, 2, 3]\n",
    "state s(1): i64\nClip c | fixed=[s + i64(1), 2, 3]\n",
    "Clip c | fixed=[1, 2]\n",
    "Clip c | fixed=[u8(255) + 4, 2, 3]\n",
    "Clip c | fixed=[255 + 4, 2, 3]\n",
];

const TO_ARRAYS_ARRAY_ATTRIBUTES: [&str; 8] = [
    "Clip c | steps=[0.5, 1, 2]\n",
    "Clip c | steps=[0.5, 1]\n",
    "Clip c | steps=[f64(0.5), 1, 2]\n",
    "state s(1)\nClip c | steps=[s, 1, 2]\n",
    "state s(1.5)\nClip c | steps=[s, 1, 2]\n",
    "Lane l | steps=[1, 2]\n",
    "Lane l | steps=[1e40, 2]\n",
    "Clip c | steps=[true, 1, 2]\n",
];

#[test]
fn a_fixed_array_attribute_binds_its_literals_elements_as_the_old_engine_does() {
    let vocabularies = [
        (
            symphonyscript_fixtures::native_vocabulary(),
            NATIVES_ARRAY_ATTRIBUTES.as_slice(),
        ),
        (
            symphonyscript_fixtures::signature_vocabulary(),
            SIGNATURES_ARRAY_ATTRIBUTES.as_slice(),
        ),
        (
            symphonyscript_fixtures::to_array_vocabulary(),
            TO_ARRAYS_ARRAY_ATTRIBUTES.as_slice(),
        ),
    ];
    for (vocabulary, programs) in vocabularies {
        let old = Compiler::new(&vocabulary)
            .expect("the vocabulary compiles")
            .on_the_old_engine();
        let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        for text in programs {
            let sources = [source(text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
    let new = Compiler::new(&symphonyscript_fixtures::native_vocabulary())
        .expect("the vocabulary compiles");
    for text in [
        "let t = [1, 2, 3]\nClip c | trio=t\n",
        "Clip c | trio=[1, 2, 3, 4][0..3]\n",
        "Clip c | seq=[1, 2]\n",
        "Clip c | rows=[[1, 2]]\n",
    ] {
        assert!(new.checked_by_new_core(&[source(text)]).is_none(), "{text}");
    }
}

const NESTED_SCOPES: [&str; 22] = [
    "fn f(p): for i in 0..3 into s from 0: s + p\nClip c:\n    Note/([0][f(2) - 6])\n    Note/([0][f(5) - 15])\n",
    "fn f(p): for i in 0..3 into s from 0: s + p\nClip c:\n    Note/([0][f(2) - 7])\n",
    "fn f(p):\n    let k = p + 1\n    for i in 0..3 into s from 0: s + k\nClip c:\n    Note/([0][f(2) - 9])\n",
    "fn f(p):\n    let k = [p, p + 1]\n    for i in 0..2 into s from 0: s + k[i]\nClip c:\n    Note/([0][f(3) - 7])\n",
    "fn h(v): v * 2\nfn f(p):\n    let k = h(p)\n    for i in 0..2 into s from 0: s + k + h(i)\nClip c:\n    Note/([0][f(3) - 14])\n",
    "fn f(p, q): for x in p into s from 0: s + x * q\nClip c:\n    Note/([0][f([1, 2], 3) - 9])\n",
    "fn f(p): for i in 0..3 into s from 0: s + p\nClip c:\n    Note/(f(true))\n",
    "fn f(p): for i in 0..3 into s from 0: s + p\nClip c:\n    Note | velocity=(f(0.25))\n    Note | velocity=(f(0.125))\n",
    "fn f(p): for i in 0..3 into s from 0: s + p\nClip c:\n    Note | velocity=(f(0.5))\n",
    "fn f(p): for i in 0..2 into s from 0: s + p + true\nClip c:\n    Note/(f(1))\n",
    "fn g(p): for i in 0..2 into s from 0: s + (for j in 0..2 into u from 0: u + p + i)\nClip c:\n    Note/([0][g(1) - 6])\n    Note/([0][g(2) - 10])\n",
    "fn f(p): for i in 0..3 into s from 0: s + p\nlet t = for i in 0..3 into a from 0: a + f(i)\nClip c:\n    Note/t\n",
    "let t = for i in 0..3 into a from 0: a + (for j in 0..2 into b from 0: b + i)\nClip c:\n    Note/([0, 0, 0, 0, 0, 0, 0][t - 6])\n",
    "let t = for i in 0..3 into a from 0: a + (for j in 0..2 into b from 0: b + a)\nClip c:\n    Note/([0][t - 7])\n",
    "let t = for i in 0..3 into a from 0:\n    let w = i * 10\n    a + (for j in 0..2 into b from 0: b + w)\nClip c:\n    Note/([0][t - 60])\n",
    "let t = for i in 0..3 into a from 0: a + (for j in 0..2 into b from 0: b + (for k in 0..2 into e from 0: e + i))\nClip c:\n    Note/([0][t - 12])\n",
    "let t = for i in 0..3 into a from 0: a + (for j in 0..2 into a from 0: a + i)\nClip c:\n    Note/([0][t - 6])\n",
    "let t = for i in 0..3 into a from 0.5: a + (for j in 0..2 into b from 0: b + i)\nClip c:\n    Note | velocity=(t)\n",
    "let t = for i in 0..3 into a from []: a + [(for j in 0..2 into b from 0: b + i)]\nClip c:\n    Note/([0][t[2] - 4])\n",
    "let t = for i in 0..2 into a from 0: a + (for j in 0..300 into b from 0: b + i)\nClip c:\n    Note/(t)\n",
    "let t = for i in 0..3 into a from 0: a + (for j in 0..2 into b from 0: b + a + true)\nClip c:\n    Note/t\n",
    "let t = for i in 0..3 into a from 0: a + (for j in 0..2 into b from 0: b + [1, 2][i])\nClip c:\n    Note/t\n",
];

#[test]
fn a_fold_body_reads_an_enclosing_fold_or_function_name_from_the_frame_that_binds_it() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in NESTED_SCOPES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}
