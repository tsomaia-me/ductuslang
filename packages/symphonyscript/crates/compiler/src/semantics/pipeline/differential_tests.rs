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

fn stop_limits() -> symphonyscript_core::LimitOverrides {
    symphonyscript_core::LimitOverrides {
        expansion_steps: Some(10),
        expansion_steps_warning: Some(5),
        ..symphonyscript_core::LimitOverrides::default()
    }
}

fn judged_under(ledger: &str) -> Compiler {
    let limits = if ledger == reviewed::LIMIT_STOP {
        stop_limits()
    } else if ledger == reviewed::LOOP_HEAD_STOP {
        symphonyscript_core::LimitOverrides {
            inlined_calls: Some(10),
            inlined_calls_warning: Some(5),
            ..symphonyscript_core::LimitOverrides::default()
        }
    } else {
        symphonyscript_core::LimitOverrides::default()
    };
    Compiler::with_limits(&symphonyscript_fixtures::vocabulary(), limits)
        .expect("the vocabulary compiles")
}

#[test]
fn the_reviewed_moves_draw_their_expected_sets_on_the_new_core_464_476() {
    for (ledger, text, _) in REVIEWED.iter().copied() {
        let found = judged_under(ledger)
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
    let programs = REVIEWED
        .iter()
        .map(|(ledger, text, _)| (*ledger, *text, false))
        .chain(
            TWIN_PASS_TYPE_MISMATCHES
                .iter()
                .map(|text| ("476", *text, true)),
        );
    let mut wrong = Vec::new();
    for (ledger, text, twin_pass) in programs {
        let new = judged_under(ledger);
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
            if open_loop_entry(text, &found, &expected) {
                moved.push(format!(
                    "476 (an open loop's no-entry-instance) {}:\n{text}",
                    corpus.name
                ));
                continue;
            }
            let (old_drawn, new_drawn) = (drawn(&expected), drawn(&found));
            match reviewed::moved(text, &old_drawn, &new_drawn) {
                Ok(classes)
                    if restated_or_kept(text, (&found, &new_drawn), (&expected, &old_drawn))
                        .is_some_and(|restated| restated || !classes.is_empty()) =>
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

const EXHAUSTIVE_CLASSES: [(&str, &[(&str, &str)]); 11] = [
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
        "state q(false)\nClip c:\n    Note/(if (1 and q) then 1 else (if [1, 2][5] then 3 else 2))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "1"),
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

const FAULTY_LINE_VALUES: [(&str, &str, &str); 7] = [
    ("if k[0] > 1 then k else e + 3]", "let-syntax", "]"),
    ("if k[0] > 1 then k else e + -3]", "let-syntax", "]"),
    ("if k[0] > 1 then k else e +  3]", "let-syntax", "]"),
    ("if k[0] > 1 then k else e + 13]", "let-syntax", "]"),
    (
        "-if k[0] > 1 then k else e + [3]",
        "unparenthesized-if",
        "if",
    ),
    (
        "- if k[0] > 1 then k else e + [3]",
        "unparenthesized-if",
        "if",
    ),
    ("e + 3]", "let-syntax", "]"),
];

#[test]
fn a_let_on_a_line_holding_a_syntax_fault_draws_no_type_fault_of_its_own_on_that_line_455() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (value, code, at) in FAULTY_LINE_VALUES {
        let items = [
            "state k([1, 2]): i32[..4]\n".to_string(),
            "let e = []\n".to_string(),
            format!("let j = {value}\n"),
            "Clip c:\n    Note/(j[^1])\n".to_string(),
        ];
        for order in [[0, 1, 2, 3], [3, 2, 1, 0]] {
            let text: String = order
                .iter()
                .filter_map(|at| items.get(*at).map(String::as_str))
                .collect();
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(
                spanned(&text, &found),
                vec![(code.to_string(), at.to_string())],
                "{text}"
            );
            let before = old.check(&sources);
            if before != found {
                let classes = reviewed::moved(&text, &drawn(&before), &drawn(&found))
                    .unwrap_or_else(|left| panic!("{text}\n  unexplained {left:?}"));
                assert!(classes.iter().all(|class| class.ledger == "455"), "{text}");
            }
        }
    }
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

fn restated_or_kept(
    text: &str,
    (found, new): (&[crate::diagnostics::Diagnostic], &[reviewed::Drawn]),
    (expected, old): (&[crate::diagnostics::Diagnostic], &[reviewed::Drawn]),
) -> Option<bool> {
    let (mine, theirs) = (kept(found, new, old), kept(expected, old, new));
    let mut restated = false;
    for (a, b) in mine.iter().zip(&theirs).filter(|(a, b)| a != b) {
        let at = drawn(std::slice::from_ref(a)).into_iter().next()?;
        if a.code != b.code || a.span != b.span || !reviewed::restated(text, old, new, &at) {
            return None;
        }
        restated = true;
    }
    (mine.len() == theirs.len()).then_some(restated)
}

fn kept(
    found: &[crate::diagnostics::Diagnostic],
    own: &[reviewed::Drawn],
    other: &[reviewed::Drawn],
) -> Vec<crate::diagnostics::Diagnostic> {
    let mut left: Vec<&reviewed::Drawn> = other.iter().collect();
    found
        .iter()
        .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::NoEntryInstance)
        .zip(own)
        .filter(|(_, at)| match left.iter().position(|kept| kept == at) {
            Some(position) => {
                left.swap_remove(position);
                true
            }
            None => false,
        })
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

const ARMS_UNDER_A_REFUSED_CONDITION: [&str; 5] = [
    "Clip c:\n    Note/(if (1.5 is [1, 2]) then 1 else (if [1, 2][5] then 2 else 3))\n",
    "let a = i32(if i32(true) then [1, 2][5] else [1, 2][5])\nClip c:\n    Note/(a)\n",
    "let a = i32(if i32(true) then [1, 2][5] else [1, 2][5])\nClip c:\n    Note | velocity=(a)\n",
    "let a = u8(if i32(true) then [1, 2][5] else 2)\nClip c:\n    Note/(a)\n",
    "let a: i32 = i32(if i32(true) then [1, 2][5] else [1, 2][5])\nClip c:\n    Note/(a)\n",
];

#[test]
fn a_choice_whose_condition_is_refused_checks_both_arms_as_a_runtime_choice_and_draws_their_index_ranges_476()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in ARMS_UNDER_A_REFUSED_CONDITION {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .expect("the new core takes the program");
        assert!(
            spanned(text, &found).contains(&("index-out-of-range".to_string(), "5".to_string())),
            "{text}"
        );
        let classes = reviewed::moved(text, &drawn(&old.check(&sources)), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "476"),
            "{text}"
        );
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

const MEMOIZED_CALLS: [&str; 11] = [
    "fn f(x): x * 4611686018427387904\nClip c:\n    Note/(f(4))\n    Note/(f(4))\n",
    "fn f(x): u8(x) + 1\nClip c:\n    Note/(f(300))\n    Note/(f(300))\n",
    "fn g(x): u8(x)\nfn f(x): g(x) + g(x)\nClip c:\n    Note/(f(300))\n    Note/(f(300))\n",
    "let big = [1, 2]\nfn f(x): x[5]\nClip c:\n    Note/(f(big))\n    Note/(f(big))\n",
    "let big = [1, 2]\nfn g(x): x[5]\nfn f(x): g(x) + 1\nClip c:\n    Note/(f(big))\n    Note/(f(big))\n",
    "fn f(x): [1, 2][x]\nClip c:\n    Note/(f(4))\n    Note/(f(4))\n    Note/(f(1))\n",
    "fn f(x): for i in 0..x into a from 0: a + i\nClip c:\n    Note/(f(2))\n    Note/(f(2))\n    Note/(f(2))\n",
    "let big = [1, 2]\nfn f(x): x[0] + x[1]\nClip c:\n    Note/(f(big))\n    Note | velocity=(f(big))\n",
    "fn f(x): for i in 0..x into a from 0: a + i\nClip c:\n    Note/(f(3))\n    Note/(f(3))\n    Note/(f(3))\n    Note/(f(3))\n",
    "fn f(p): for i in 0..4 into s from [p]: s + s\nClip c:\n    Note/(f(1)[0])\n",
    "fn h(s): s + s\nfn f(p): for i in 0..4 into s from [p]: h(s)\nClip c:\n    Note/(f(1)[0])\n",
];

type Stop = (
    &'static str,
    [(&'static str, std::ops::Range<u32>, usize); 2],
);

const MEMOIZED_STOP: Stop = (
    "fn g(x): for i in 0..x into a from 0: a + i\nfn f(x): g(x) + g(x)\nClip c:\n    Note/(f(2))\n    Note/(f(2))\n    Note/(f(2))\n",
    [("expansion-limit", 9..43, 2), ("limit-warning", 9..43, 2)],
);

#[test]
fn a_memoized_call_draws_its_faults_at_each_sites_chain_and_charges_its_iterations_again() {
    let overrides = symphonyscript_core::LimitOverrides {
        value_elements: Some(8),
        value_elements_warning: Some(4),
        ..stop_limits()
    };
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::with_limits(&vocabulary, overrides)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::with_limits(&vocabulary, overrides).expect("the vocabulary compiles");
    for text in MEMOIZED_CALLS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
    let (text, expected) = MEMOIZED_STOP;
    let found = new
        .checked_by_new_core(&[source(text)])
        .unwrap_or_else(|| panic!("the new core takes {text}"));
    let found: Vec<_> = found
        .iter()
        .map(|d| (d.code.to_string(), d.span.range.clone(), d.related.len()))
        .collect();
    let expected: Vec<_> = expected
        .iter()
        .map(|(code, range, chain)| (code.to_string(), range.clone(), *chain))
        .collect();
    assert_eq!(found, expected, "{text}");
    assert_eq!(
        reviewed(text).map(|(ledger, _)| ledger),
        Some(reviewed::LIMIT_STOP)
    );
}

const FOLD_FAULTS_PER_SITE: [(&str, &str, usize); 4] = [
    (
        "fn f(p): for i in 0..2 into s from 0: s + true\nClip c:\n    Note/(f(1))\n    Note/(f(2))\n",
        "s + true",
        2,
    ),
    (
        "fn f(p): for i in 0..2 into s from 0: s + true\nClip c:\n    Note/(f(1))\n    Note/(f(2))\n    Note/(f(3))\n",
        "s + true",
        3,
    ),
    (
        "fn g(p): for i in 0..2 into s from 0: s + true\nfn f(p): g(p) + 1\nClip c:\n    Note/(f(1))\n    Note/(f(2))\n",
        "s + true",
        2,
    ),
    (
        "fn f(p):\n    let k = p + 1\n    for i in 0..2 into s from 0: s + k + true\nClip c:\n    Note/(f(1))\n    Note | velocity=(f(2))\n",
        "s + k + true",
        2,
    ),
];

const FOLD_FAULT_TWINS: [&str; 5] = [
    "fn f(p): p + true\nClip c:\n    Note/(f(1))\n    Note/(f(2))\n",
    "fn f(p): for i in 0..2 into s from 0: s + true\nClip c:\n    Note/(f(1))\n",
    "let t = for i in 0..2 into s from 0: s + true\nClip c:\n    Note/(t)\n    Note/(t)\n",
    "fn f(p): for i in 0..2 into s from 0: s + [1, 2][p]\nClip c:\n    Note/(f(5))\n    Note/(f(5))\n",
    "fn f(p):\n    let k = p + 1\n    for i in 0..2 into s from 0: s + [1, 2][5] + k\nClip c:\n    Note/(f(1))\n    Note/(f(2))\n",
];

#[test]
fn a_fault_inside_a_fold_in_a_function_body_draws_at_each_call_sites_chain_479() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, span, sites) in FOLD_FAULTS_PER_SITE {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut chains: Vec<u32> = found
            .iter()
            .filter(|d| {
                text.get(d.span.range.start as usize..d.span.range.end as usize) == Some(span)
            })
            .filter_map(|d| d.related.last().map(|site| site.span.range.start))
            .collect();
        chains.dedup();
        assert_eq!(chains.len(), sites, "{text}");
        let before = old.check(&sources);
        let classes = reviewed::moved(text, &drawn(&before), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "479"),
            "{text}"
        );
        assert_eq!(
            kept(&found, &drawn(&found), &drawn(&before)),
            kept(&before, &drawn(&before), &drawn(&found)),
            "{text}"
        );
    }
    for text in FOLD_FAULT_TWINS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const REFUSED_BODIES: [&str; 12] = [
    "fn f(p): [1, 2][p] + true\nClip c:\n    Note/(f(5))\n",
    "fn f(p): true + [1, 2][p]\nClip c:\n    Note/(f(5))\n",
    "fn f(p): (true + 1) + [1, 2][p]\nClip c:\n    Note/(f(5))\n",
    "fn f(p): [1, 2][p] + (true + 1)\nClip c:\n    Note/(f(5))\n",
    "fn f(p): [p] * 0 + [1, 2][p]\nClip c:\n    Note/(f(5)[0])\n",
    "fn g(q): [1, 2][q] + true\nfn f(p): g(p + 3)\nClip c:\n    Note/(f(2))\n    Note/(f(0))\n",
    "fn f(p, r): [1, 2][p] + r\nClip c:\n    Note/(f(5, true))\n",
    "state s(1)\nfn f(p): [1, 2][p] + true\nClip c:\n    Note/(f(s))\n",
    "fn f(p): [1, 2][p + 300000000000] + true\nClip c:\n    Note/(f(5))\n",
    "fn f(p): [1, 2][p] + true\nClip c:\n    Note/(f(1))\n",
    "fn f(p): (true + 1) + (p + 300000000000)\nClip c:\n    Note/(f(5))\n",
    "let k = 5\nClip c:\n    Note/(([1] * 0 + [1, 2][k])[0])\n",
];

#[test]
fn a_call_whose_body_holds_a_refused_operator_checks_its_operands_with_the_sites_arguments() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in REFUSED_BODIES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const CHAINS_OF_BODY_READS: [&str; 9] = [
    "fn f(n): n + [1, 2][..0][0]\nClip c:\n    Note/(f(1))\n",
    "fn f(n): [1, 2][..0][0]\nClip c:\n    Note/(f(1))\n    Note/(f(2))\n",
    "fn f(n): n + [1, 2][..0][0]\nClip c:\n    Note/(f(1) + 1)\n",
    "fn f(n): [1, 2][..n][0]\nClip c:\n    Note/(f(0))\n",
    "let y = [1, 2][..0][0]\nClip c:\n    Note/(y)\n",
    "let xs = [1, true]\nfn f(n): n + xs[5]\nClip c:\n    Note/(f(1))\n",
    "let xs = [1, true]\nfn f(n): n + xs[0]\nClip c:\n    Note/(f(1))\n    Note/(f(2))\n",
    "Clip c:\n    Note/(f(1))\nfn f(n): n + xs[^5]\nlet xs = [1, true]\n",
    "let xs = [1, 2]\nlet ys = xs[5]\nfn f(n): n + ys\nClip c:\n    Note/(f(1))\n",
];

#[test]
fn a_top_level_lets_fault_read_in_a_body_draws_unchained_and_a_helped_fault_in_a_call_carries_its_chain()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in CHAINS_OF_BODY_READS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const RUNTIME_NON_BOOL_CONDITIONS: [&str; 4] = [
    "state q(0): i32\nClip c:\n    Note/(if q then q else [1, 2][5])\n",
    "state q(0)\nClip c:\n    Note/(if q then [1, 2][5] else 3)\n",
    "Clip c:\n    Note/(if 1 then 3 else [1, 2][5])\n",
    "state q(0)\nClip c:\n    Note/(if q then 3 else 4)\n",
];

#[test]
fn a_runtime_condition_of_another_type_than_bool_checks_both_arms_as_the_old_engine_does() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in RUNTIME_NON_BOOL_CONDITIONS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

const OWN_TYPE_OPERATIONS: [&str; 23] = [
    "Clip c:\n    Note/(u8(300000000000 / 300000000000))\n",
    "state q(0.5): f32\nClip c:\n    Note/(q - 300000000000)\n",
    "state q(0.5): f32\nClip c:\n    Note/(300000000000 - q)\n",
    "state q(0.5): f32\nClip c:\n    Note/((q - 300000000000) / 2)\n",
    "state q(0.5): f32\nClip c:\n    Note/(q * 1e300)\n",
    "state q(0.5): f32\nClip c:\n    Note/(q + 1)\n",
    "state q(0.5): f32\nClip c:\n    Note/(q / 0)\n",
    "state q(0.5)\nClip c:\n    Note/(q - 300000000000)\n",
    "state q(0.5)\nClip c:\n    Note/(q / 300000000000)\n",
    "state q(0): i32\nClip c:\n    Note/(q / 300000000000)\n",
    "state q(0): u8\nClip c:\n    Note/(300000000000 / q)\n",
    "state q(0): i64\nClip c:\n    Note/(q / 300000000000)\n",
    "state q(0)\nClip c:\n    Note/(q / 300000000000)\n",
    "state x(1)\nClip c:\n    Note/(x + 300000000000 + 1.5)\n",
    "state q(0): i32\nClip c:\n    Note/(q is 300000000000)\n",
    "state q(0): u8\nClip c:\n    Note/(q < 300000000000)\n",
    "state q(0.5): f32\nClip c:\n    Note/(q is 300000000000)\n",
    "state q(0.5)\nClip c:\n    Note/(q is 300000000000)\n",
    "let q = 0.5\nClip c:\n    Note/(q is 300000000000)\n",
    "let q = 0.5\nClip c:\n    Note/(if (q is 300000000000) then 1 else 2)\n",
    "let q = 1\nClip c:\n    Note/(q / 300000000000)\n",
    "let x = i64(1) / 300000000000\nClip c:\n    Note/(1)\n",
    "state q(0): i32\nClip c:\n    Note | velocity=(q / 300000000000)\n",
];

#[test]
fn a_refused_operation_over_a_cell_checks_its_literals_in_the_operations_own_type() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for text in OWN_TYPE_OPERATIONS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

type Summarized = (
    &'static str,
    &'static [(&'static str, &'static str, &'static str)],
);

const ANNOTATION_AND_CHOICE_FAULTS: [Summarized; 8] = [
    (
        "state q(0): u8\nlet a: i32 = q - 300000000000\nClip c:\n    Note/(a)\n",
        &[
            (
                "does-not-fit",
                "300000000000",
                "this value does not fit `i32`",
            ),
            (
                "reactive-let",
                "a",
                "`a` is a `let`, a compile-time value, and its value reads a cell",
            ),
        ],
    ),
    (
        "state q(0): u8\nlet a: i32 = q - 300\nClip c:\n    Note/(a)\n",
        &[(
            "reactive-let",
            "a",
            "`a` is a `let`, a compile-time value, and its value reads a cell",
        )],
    ),
    (
        "state q(0): i32\nlet a: u8 = q - 300\nClip c:\n    Note/(a)\n",
        &[
            ("does-not-fit", "300", "this value does not fit `u8`"),
            (
                "reactive-let",
                "a",
                "`a` is a `let`, a compile-time value, and its value reads a cell",
            ),
        ],
    ),
    (
        "Clip c:\n    Note/(if 1 then 1 else (true or true))\n",
        &[
            ("condition-type", "1", "the condition is a `bool`"),
            (
                "type-mismatch",
                "if 1 then 1 else (true or true)",
                "the branches of an `if` hold values of different types",
            ),
        ],
    ),
    (
        "Clip c:\n    Note/(if 1 then 1 else 2)\n",
        &[("condition-type", "1", "the condition is a `bool`")],
    ),
    ("Clip c:\n    Note/(if true then 1 else 2)\n", &[]),
    (
        "state q(0): i32\nClip c:\n    Note/([1, 2][q..])\n",
        &[(
            "type-mismatch",
            "[1, 2][q..]",
            "`pitch` holds one `i32`, and this value is a variable array",
        )],
    ),
    (
        "Clip c:\n    Note/([1, 2][1..])\n",
        &[(
            "type-mismatch",
            "[1, 2][1..]",
            "`pitch` holds one `i32`, and this value is an array",
        )],
    ),
];

#[test]
fn a_reactive_let_computes_in_its_annotations_type_and_a_refused_condition_keeps_the_branches_fault()
 {
    assert_summarized(&ANNOTATION_AND_CHOICE_FAULTS);
}

fn assert_summarized(table: &[Summarized]) {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for (text, expected) in table {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut seen: Vec<(String, String, String)> = found
            .iter()
            .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::NoEntryInstance)
            .map(|diagnostic| {
                let range =
                    diagnostic.span.range.start as usize..diagnostic.span.range.end as usize;
                (
                    diagnostic.code.to_string(),
                    text.get(range).unwrap_or_default().to_string(),
                    diagnostic.summary.clone(),
                )
            })
            .collect();
        seen.sort();
        let mut wanted: Vec<(String, String, String)> = expected
            .iter()
            .map(|(code, span, summary)| {
                (
                    (*code).to_string(),
                    (*span).to_string(),
                    (*summary).to_string(),
                )
            })
            .collect();
        wanted.sort();
        assert_eq!(seen, wanted, "{text}");
    }
}

const CONDITION: &str = "the condition is a `bool`";
const ARRAY_CONDITION: &str = "the condition of an `if` is a `bool`, not an array";
const WRONG_I32: &str = "this value has the wrong type here; `i32` is expected";
const BRANCHES: &str = "the branches of an `if` hold values of different types";

const CHOICE_TYPE_FAULTS: [Summarized; 13] = [
    (
        "Clip c:\n    Note/(if 1 then true else true)\n",
        &[
            ("condition-type", "1", CONDITION),
            ("type-mismatch", "if 1 then true else true", WRONG_I32),
        ],
    ),
    (
        "Clip c:\n    Note/(if true then true else true)\n",
        &[("type-mismatch", "if true then true else true", WRONG_I32)],
    ),
    (
        "let q = 1\nClip c:\n    Note/(if true then true else (if 1 then 2 else 3))\n",
        &[
            ("condition-type", "1", CONDITION),
            (
                "type-mismatch",
                "if true then true else (if 1 then 2 else 3)",
                BRANCHES,
            ),
        ],
    ),
    (
        "Clip c:\n    Note/(if ([1, 2][5][i32(true)]) then (300000000000 is 1.5) else (1.5 or 1.5))\n",
        &[
            (
                "conversion-type",
                "i32(true)",
                "a `bool` or an enum value is not converted",
            ),
            (
                "index-out-of-range",
                "5",
                "the index 5 is outside the array of length 2",
            ),
            ("not-an-array", "[1, 2][5]", "only an array is indexed"),
            ("operand-type", "1.5 or 1.5", "`and` and `or` take `bool`s"),
            (
                "type-mismatch",
                "if ([1, 2][5][i32(true)]) then (300000000000 is 1.5) else (1.5 or 1.5)",
                WRONG_I32,
            ),
        ],
    ),
    (
        "Clip c:\n    Note/(if [1] then true else 1)\n",
        &[
            ("condition-type", "if [1] then true else 1", ARRAY_CONDITION),
            ("type-mismatch", "if [1] then true else 1", BRANCHES),
        ],
    ),
    (
        "state s(0)\nClip c:\n    Note/(if s then true else true)\n",
        &[
            ("condition-type", "s", CONDITION),
            ("type-mismatch", "if s then true else true", WRONG_I32),
        ],
    ),
    (
        "state s(false)\nClip c:\n    Note/(if s then true else true)\n",
        &[("type-mismatch", "if s then true else true", WRONG_I32)],
    ),
    (
        "Clip c:\n    Note/((if 1 then [1] else [2])[1.5 < 1..])\n",
        &[
            ("condition-type", "1", CONDITION),
            ("index-type", "1.5 < 1", "a slice bound is an integer"),
            (
                "type-mismatch",
                "(if 1 then [1] else [2])[1.5 < 1..]",
                "`pitch` holds one `i32`, and this value is an array",
            ),
        ],
    ),
    (
        "state q(0): i32\nClip c:\n    Note/(1 and (if true then 300000000000 else q))\n",
        &[
            (
                "does-not-fit",
                "300000000000",
                "this value does not fit `i32`",
            ),
            (
                "operand-type",
                "1 and (if true then 300000000000 else q)",
                "`and` and `or` take `bool`s",
            ),
            (
                "type-mismatch",
                "1 and (if true then 300000000000 else q)",
                WRONG_I32,
            ),
        ],
    ),
    (
        "state q(0.5): f64\nClip c:\n    Note/(true and (if true then 300000000000 else q))\n",
        &[
            (
                "type-mismatch",
                "if true then 300000000000 else q",
                "this value has the wrong type here; `bool` is expected",
            ),
            (
                "type-mismatch",
                "true and (if true then 300000000000 else q)",
                WRONG_I32,
            ),
        ],
    ),
    (
        "state q(0): i32\nClip c:\n    Note/(true and (if true then 300 else q))\n",
        &[
            (
                "type-mismatch",
                "if true then 300 else q",
                "this value has the wrong type here; `bool` is expected",
            ),
            (
                "type-mismatch",
                "true and (if true then 300 else q)",
                WRONG_I32,
            ),
        ],
    ),
    (
        "state q(0): i32\nClip c:\n    Note/(if ([1, 2][5] - [1, 2]) then ([1, 2] is [1, 2]) else (if q then [1, 2] else [1, 2][5]))\n",
        &[
            (
                "array-operand",
                "[1, 2] is [1, 2]",
                "only `+` takes arrays, which it joins; every other operator takes single values",
            ),
            (
                "array-operand",
                "[1, 2][5] - [1, 2]",
                "only `+` takes arrays, which it joins; every other operator takes single values",
            ),
            ("condition-type", "q", CONDITION),
            (
                "index-out-of-range",
                "5",
                "the index 5 is outside the array of length 2",
            ),
            (
                "type-mismatch",
                "if q then [1, 2] else [1, 2][5]",
                "the branches hold arrays of different shapes",
            ),
        ],
    ),
    (
        "state q(0): i32\nClip c:\n    Note/(if true then ([1, 2] is [1, 2]) else (if q then [1, 2] else [1, 2][5]))\n",
        &[
            (
                "array-operand",
                "[1, 2] is [1, 2]",
                "only `+` takes arrays, which it joins; every other operator takes single values",
            ),
            ("condition-type", "q", CONDITION),
            ("type-mismatch", "[1, 2] is [1, 2]", WRONG_I32),
            (
                "type-mismatch",
                "if q then [1, 2] else [1, 2][5]",
                "the branches hold arrays of different shapes",
            ),
        ],
    ),
];

#[test]
fn a_choice_keeps_its_arms_type_whatever_its_condition_holds_and_its_arm_is_checked_in_its_own_type()
 {
    assert_summarized(&CHOICE_TYPE_FAULTS);
}

fn open_loop_entry(
    text: &str,
    found: &[crate::diagnostics::Diagnostic],
    expected: &[crate::diagnostics::Diagnostic],
) -> bool {
    use crate::diagnostics::Code;
    let entry = |set: &[crate::diagnostics::Diagnostic]| {
        set.iter()
            .any(|diagnostic| diagnostic.code == Code::NoEntryInstance)
    };
    let rest = |set: &[crate::diagnostics::Diagnostic]| -> Vec<crate::diagnostics::Diagnostic> {
        set.iter()
            .filter(|diagnostic| diagnostic.code != Code::NoEntryInstance)
            .cloned()
            .collect()
    };
    reviewed::runs_no_iteration(text, &drawn(found))
        && entry(found)
        && !entry(expected)
        && rest(found) == rest(expected)
}

#[test]
fn a_no_entry_instance_alone_moves_only_for_a_program_whose_placement_for_runs_no_iteration_476() {
    use crate::diagnostics::Code;
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let entry = old
        .check(&[source("Clip c:\n    Note/1\n")])
        .into_iter()
        .find(|diagnostic| diagnostic.code == Code::NoEntryInstance)
        .expect("a program of no entry kind draws no-entry-instance");
    for (text, moves) in [
        ("Clip c:\n    for x in zz:\n        Note/x\n", true),
        ("Clip c:\n    for x in true:\n        Note/x\n", true),
        (
            "Clip c:\n    for x in [1, 2][0..300000000000000000000]:\n        Note/x\n",
            true,
        ),
        (
            "state s(1)\nlet ys = [s]\nClip c:\n    for x in ys:\n        Note/x\n",
            true,
        ),
        ("Clip c:\n    for x in [1, 2]:\n        Note/x\n", false),
        ("Clip c:\n    for x in [1, 2]:\n        Note/zz\n", false),
        (
            "Clip c:\n    Note/zz\n    for x in [1, 2]:\n        Note/x\n",
            false,
        ),
        (
            "let ys = [1, 2]\nClip c:\n    Note/(1 + true)\n    for x in ys:\n        Note/x\n",
            false,
        ),
    ] {
        let expected: Vec<_> = old
            .check(&[source(text)])
            .into_iter()
            .filter(|diagnostic| diagnostic.code != Code::NoEntryInstance)
            .collect();
        let mut found = expected.clone();
        found.insert(0, entry.clone());
        assert_eq!(open_loop_entry(text, &found, &expected), moves, "{text}");
    }
}

const LINE_FAULT_LETS: [(&str, crate::diagnostics::Code); 5] = [
    ("let $l = level * 2", crate::diagnostics::Code::LetSyntax),
    ("let l = level $ 2", crate::diagnostics::Code::LetSyntax),
    (
        "let l = level * ",
        crate::diagnostics::Code::ExpressionSyntax,
    ),
    ("let l = level * 2 $", crate::diagnostics::Code::LetSyntax),
    ("let l = level * 2;", crate::diagnostics::Code::Semicolon),
];

fn orders(items: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    let mut order: Vec<usize> = (0..items.len()).collect();
    let mut counts = vec![0usize; items.len()];
    found.push(order.iter().map(|at| items[*at].as_str()).collect());
    let mut at = 0;
    while at < items.len() {
        if counts[at] < at {
            let other = if at % 2 == 0 { 0 } else { counts[at] };
            order.swap(other, at);
            found.push(order.iter().map(|at| items[*at].as_str()).collect());
            counts[at] += 1;
            at = 0;
        } else {
            counts[at] = 0;
            at += 1;
        }
    }
    found
}

fn line_fault_programs(line: &str) -> Vec<String> {
    let alone = [
        "state level(0.0)\n".to_string(),
        format!("{line}\n"),
        "Clip c | gain=l:\n    Note\n".to_string(),
    ];
    let grouped = [
        "state level(0.0)\n".to_string(),
        format!("{line}\n"),
        "state held = if l > 1 then l else held.past(1, 0.0)\n".to_string(),
        "Clip c | gain=held:\n    Note\n".to_string(),
    ];
    let mut programs = orders(&alone);
    programs.extend(orders(&grouped));
    programs
}

#[test]
fn a_let_whose_line_holds_a_syntax_fault_at_its_start_middle_or_end_draws_no_reactive_let_inside_or_outside_a_group_in_every_order_491()
 {
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let codes = |text: &str| -> Vec<crate::diagnostics::Code> {
        new.checked_by_new_core(&[source(text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"))
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    };
    for text in line_fault_programs("let l = level * 2") {
        assert!(
            codes(&text).contains(&crate::diagnostics::Code::ReactiveLet),
            "{text}"
        );
    }
    for (line, fault) in LINE_FAULT_LETS {
        for text in line_fault_programs(line) {
            let found = codes(&text);
            assert!(found.contains(&fault), "{text}\n  {found:?}");
            assert!(
                !found.contains(&crate::diagnostics::Code::ReactiveLet),
                "{text}\n  {found:?}"
            );
        }
    }
}

#[test]
fn a_missing_closer_or_block_reported_at_a_lets_first_token_counts_against_its_own_line_so_the_let_draws_reactive_let_in_every_order_492()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let reactive_at_l = |text: &str, found: &[crate::diagnostics::Diagnostic]| {
        let at = text.find("let l").map_or(0, |at| at + 4);
        found.iter().any(|diagnostic| {
            diagnostic.code == crate::diagnostics::Code::ReactiveLet
                && diagnostic.span.range.start as usize == at
        })
    };
    let valid = [
        "state level(0.0)\n".to_string(),
        "let l = level * 2\n".to_string(),
        "Clip c:\n    Note/(1)\n".to_string(),
        "Clip d:\n    Note\n".to_string(),
    ];
    let faulty = [
        "state level(0.0)\n".to_string(),
        "let l = level * 2\n".to_string(),
        "Clip c:\n    Note/(1\n".to_string(),
        "Clip d:\n".to_string(),
    ];
    let own = [
        "state level(0.0)\n".to_string(),
        "let l = level * 2\n".to_string(),
        "let m = (level * 2\n".to_string(),
        "Clip c:\n    Note/(1\n".to_string(),
    ];
    let mut programs = 0;
    for items in [&valid, &faulty, &own] {
        for text in orders(items) {
            programs += 1;
            let found = new
                .checked_by_new_core(&[source(&text)])
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert!(reactive_at_l(&text, &found), "{text}\n  {found:?}");
            assert_eq!(found, old.check(&[source(&text)]), "{text}");
            if items == &own {
                let at = text.find("let m").map_or(0, |at| at + 4);
                assert!(
                    !found.iter().any(|diagnostic| {
                        diagnostic.code == crate::diagnostics::Code::ReactiveLet
                            && diagnostic.span.range.start as usize == at
                    }),
                    "{text}\n  {found:?}"
                );
            }
        }
    }
    assert_eq!(programs, 72);
}

#[test]
fn a_fault_raised_at_a_layout_token_counts_against_the_line_that_token_ends_so_the_let_beside_it_draws_reactive_let_in_every_order_496()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let cell = "state i(1)\n".to_string();
    let read = "let s = i + 1\n".to_string();
    let valid = "Clip c:\n    Note/(1)\n".to_string();
    let at_dedent = "Clip c:\n    state Note/(1)\n".to_string();
    let at_newline = "Clip e | gain=\n".to_string();
    let at_operand = "Clip f | gain=1 +\n".to_string();
    let shortest = [
        "state i(1)\n".to_string(),
        "let xs = [10, 20, 30, 40]\n".to_string(),
        "let s = xs[i..^1] + xs[..1]\n".to_string(),
        "Clip c:\n    state Note/(s[0] + s.size)\n".to_string(),
    ];
    let sets: Vec<Vec<String>> = vec![
        vec![cell.clone(), read.clone(), valid],
        vec![cell.clone(), read.clone(), at_dedent.clone()],
        vec![cell.clone(), read.clone(), at_newline.clone()],
        vec![cell.clone(), read.clone(), at_operand.clone()],
        vec![
            cell.clone(),
            read.clone(),
            at_dedent.clone(),
            at_newline.clone(),
        ],
        vec![cell, read, at_operand, at_dedent],
        shortest.to_vec(),
    ];
    let mut programs = 0;
    for items in &sets {
        for text in orders(items) {
            programs += 1;
            let found = new
                .checked_by_new_core(&[source(&text)])
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            let at = text.find("let s").map_or(0, |at| at + 4);
            assert!(
                found.iter().any(|diagnostic| {
                    diagnostic.code == crate::diagnostics::Code::ReactiveLet
                        && diagnostic.span.range.start as usize == at
                }),
                "{text}\n  {found:?}"
            );
            assert_eq!(found, old.check(&[source(&text)]), "{text}");
        }
    }
    assert_eq!(programs, 96);
}

type OpenGroup = (
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
);

const OPEN_HISTORY_GROUPS: [OpenGroup; 5] = [
    (
        &[
            "state n(0)\n",
            "state r([0, 0]) = (b.past(1) + b.past(1) + [n])[3..]\n",
            "state b = r\n",
            "state t: i32[2] = r\n",
        ],
        &[],
    ),
    (
        &[
            "state n(0)\n",
            "state r([0, 0]) = (b.past(1) + b.past(2) + [n])[3..]\n",
            "state b = r\n",
            "state t: i32[2] = r\n",
        ],
        &[],
    ),
    (
        &[
            "state n(0)\n",
            "state r([0, 0]) = (b.past(1) + c.past(1) + [n])[3..]\n",
            "state b = r\n",
            "state c = r\n",
            "state t: i32[2] = r\n",
        ],
        &[],
    ),
    (
        &[
            "state n(0)\n",
            "state r([0, 0]) = (b.past(1) + b.past(1) + [n])[3..]\n",
            "state b = r\n",
            "state t: i32[3] = r\n",
        ],
        &[("type-mismatch", "r")],
    ),
    (
        &[
            "state n(0)\n",
            "state r([0, 0]) = (b.past(1) + b.past(1))[2..]\n",
            "state b = r\n",
            "state t: i32[2] = r\n",
        ],
        &[("unrun-cell", "r")],
    ),
];

#[test]
fn a_groups_history_read_with_neither_fallback_nor_init_takes_its_cells_settled_shape_in_every_order_461()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut programs = 0;
    for (items, wanted) in OPEN_HISTORY_GROUPS {
        let items: Vec<String> = items.iter().map(|item| (*item).to_string()).collect();
        let mut wanted: Vec<(String, String)> = wanted
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        for text in orders(&items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            let first = if text.find("state b").unwrap_or(0) < text.find("state r").unwrap_or(0) {
                "b"
            } else {
                "r"
            };
            let wanted: Vec<(String, String)> = wanted
                .iter()
                .map(|(code, span)| {
                    let span = if code == "unrun-cell" {
                        first.to_string()
                    } else {
                        span.clone()
                    };
                    (code.clone(), span)
                })
                .collect();
            assert_eq!(spanned(&text, &found), wanted, "{text}");
            let before = old.check(&sources);
            if before != found {
                let classes = reviewed::moved(&text, &drawn(&before), &drawn(&found))
                    .unwrap_or_else(|left| panic!("{text}\n  unexplained {left:?}"));
                let ledger = if wanted.iter().any(|(code, _)| code == "unrun-cell") {
                    "476"
                } else {
                    "461"
                };
                assert!(classes.iter().all(|class| class.ledger == ledger), "{text}");
            }
            programs += 1;
        }
    }
    assert_eq!(programs, 24 * 4 + 120);
}

const ARRAY_GROUPS: [&str; 16] = [
    "state x(1)\nstate h = h.past(1, [0, 0])[1..] + [x]\nClip c:\n    Note/(h[0])\n",
    "state x(1)\nstate h = h.past(1, [0, 0]) + [x]\nClip c:\n    Note/(h[0])\n",
    "state x(1)\nstate a = (b.past(1, []) + [x])[^4..]\nstate b = a\nClip c:\n    Note/(a[0] + b[0])\n",
    "state x(1)\nstate a = b.past(1, [0]) + [x]\nstate b = a.past(1, [0])\nClip c:\n    Note/(a[0])\n",
    "state x(1)\nstate a([0, 0]) = (a.past(1) + [x])[1..]\nClip c:\n    Note/(a[0])\n",
    "state x(1)\nstate a([0, 0]) = a.past(1) + [x]\nClip c:\n    Note/(a[0])\n",
    "state x(1)\nstate h = (h.past(1, []) + [x, x])[^3..]\nstate t: i32[..2] = h\nClip c:\n    Note/(t[0])\n",
    "state x(1)\nstate h = (h.past(1, [1.5]) + [x])[^3..]\nClip c:\n    Note/(h[0])\n",
    "state x(1.5)\nstate h = (h.past(1, []) + [x])[^3..]\nClip c:\n    Note | velocity=(h[0])\n",
    "state x(1)\nstate h = if x > 0 then h.past(1, [0]) else [x, x]\nClip c:\n    Note/(h[0])\n",
    "state x(1)\nstate h = h.past(1, 0) + [x]\nClip c:\n    Note/(h[0])\n",
    "state x(1)\nstate h = (h.past(1, []) + [x])[^3..]\nstate g = (h.past(2, [1, 2, 3, 4]) + [x])\nClip c:\n    Note/(g[0])\n",
    "state x(1)\nstate a([0, 0, 0]) = (a.past(1) + [x])[^2..]\nClip c:\n    Note/(a[0])\n",
    "state go(0): i32\nstate t([1, 2]) = r.past(1, [go, go])\nstate r = [t[1], t] + 1\nClip c:\n    Note/(r[0])\n",
    "state t([1, 2, 3]) = r.past(1)\nstate r = [t[1], t[0] + go]\nstate go(0): i32\nClip c:\n    Note/(t[0] + r[1])\n",
    "state t([]) = r.past(1)\nstate r = (t + [go])[^4..]\nstate go(0): i32\nClip c:\n    Note\n",
];

#[test]
fn a_group_of_cells_holding_arrays_settles_fixedness_and_capacities_together_as_the_old_engine_does()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for program in ARRAY_GROUPS {
        let mut items: Vec<String> = Vec::new();
        for line in program.lines() {
            match items.last_mut() {
                Some(item) if line.starts_with(' ') => {
                    item.push_str(line);
                    item.push('\n');
                }
                _ => items.push(format!("{line}\n")),
            }
        }
        for text in orders(&items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
}

#[test]
fn a_let_reading_the_history_of_another_member_of_a_group_draws_what_the_old_engine_draws() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let programs: [&[&str]; 2] = [
        &[
            "state x(1)\n",
            "let k = b.past(1, 0)\n",
            "state h = k + x\n",
            "state b = h\n",
            "Clip c:\n    Note/(h)\n",
        ],
        &[
            "state x(1)\n",
            "let k = b.past(1, [0])\n",
            "state h = (k + [x])[^2..]\n",
            "state b = h\n",
        ],
    ];
    for items in programs {
        let items: Vec<String> = items.iter().map(|item| (*item).to_string()).collect();
        for text in orders(&items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
            assert!(
                found
                    .iter()
                    .any(|diagnostic| diagnostic.code == crate::diagnostics::Code::ReactiveLet),
                "{text}"
            );
        }
    }
}

#[test]
fn a_fallback_of_another_shape_than_its_groups_settled_cell_draws_what_the_old_engine_draws() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let programs = [
        "state note(60)\nstate recent = (recent.past(1, []) + [note])[^8]\nClip c:\n    Note/(recent[^1])\n",
        "state note(60)\nstate recent = (recent.past(1, 0) + [note])[^8..]\nClip c:\n    Note/(recent[^1])\n",
        "state note(60)\nstate recent = recent.past(1, [1]) + note\nClip c:\n    Note/(recent)\n",
        "state note(60)\nstate recent = (recent.past(1, []) + [note])[^8..]\nClip c:\n    Note/(recent[^1])\n",
    ];
    for text in programs {
        let sources = [source(text)];
        if let Some(found) = new.checked_by_new_core(&sources) {
            assert_eq!(found, old.check(&sources), "{text}");
        }
        assert_eq!(new.check(&sources), old.check(&sources), "{text}");
    }
}

type VariableLet = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const VARIABLE_LETS: [VariableLet; 14] = [
    ("let xs: i32[..4] = [1, 2]", "xs[1]", &[]),
    (
        "let xs: i32[..4] = [1, 2]",
        "xs[3]",
        &[("index-out-of-range", "3")],
    ),
    (
        "let xs: i32[..1] = [1, 2]",
        "xs[0]",
        &[("capacity-exceeded", "let xs: i32[..1] = [1, 2]\n")],
    ),
    (
        "let xs: i32[..2] = [1, 2] + [3]",
        "xs[0]",
        &[("capacity-exceeded", "let xs: i32[..2] = [1, 2] + [3]\n")],
    ),
    (
        "let xs: i32[..2] = for i in 0..3 into a from []: a + [i]",
        "xs[0]",
        &[(
            "capacity-exceeded",
            "let xs: i32[..2] = for i in 0..3 into a from []: a + [i]\n",
        )],
    ),
    (
        "let xs: i32[..4] = [1.5, 2]",
        "xs[0]",
        &[("type-mismatch", "1.5")],
    ),
    (
        "let xs: i32[..4] = [1.5, 2] + [3]",
        "xs[0]",
        &[("type-mismatch", "[1.5, 2] + [3]")],
    ),
    (
        "let xs: u8[..4] = [1, 300]",
        "xs[0]",
        &[("does-not-fit", "300")],
    ),
    (
        "let xs: u8[..4] = [1, 2] + [300]",
        "xs[0]",
        &[("does-not-fit", "300")],
    ),
    (
        "let xs: i32[..4] = [[1], [2]]",
        "xs[0]",
        &[("type-mismatch", "let xs: i32[..4] = [[1], [2]]\n")],
    ),
    (
        "let xs: i32[..4] = 5",
        "xs",
        &[
            ("type-mismatch", "let xs: i32[..4] = 5\n"),
            ("type-mismatch", "xs"),
        ],
    ),
    (
        "let xs: i32[..1] = [1, 2]",
        "xs[1] + true",
        &[
            ("capacity-exceeded", "let xs: i32[..1] = [1, 2]\n"),
            ("operand-type", "xs[1] + true"),
        ],
    ),
    ("let xs: i32[..4] = []", "(xs + [1])[0]", &[]),
    ("let xs: i32[..4] = [1, 2]", "(xs + [1])[2]", &[]),
];

#[test]
fn a_let_annotated_with_a_variable_array_holds_its_value_checks_its_capacity_and_elements_and_keeps_its_type()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (declaration, read, wanted) in VARIABLE_LETS {
        let mut wanted: Vec<(String, String)> = wanted
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        let items = [
            format!("{declaration}\n"),
            format!("Clip c:\n    Note/({read})\n"),
        ];
        for text in [
            items.concat(),
            [items[1].clone(), items[0].clone()].concat(),
        ] {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(spanned(&text, &found), wanted, "{text}");
            let before = old.check(&sources);
            if before != found {
                let classes = reviewed::moved(&text, &drawn(&before), &drawn(&found))
                    .unwrap_or_else(|left| panic!("{text}\n  unexplained {left:?}"));
                assert!(classes.iter().all(|class| class.ledger == "476"), "{text}");
            }
        }
    }
    let reactive =
        "state hh([1]): i32[..4]\nlet xs: u8[..4] = hh + [300]\nClip c:\n    Note/(xs[0])\n";
    let sources = [source(reactive)];
    assert_eq!(new.checked_by_new_core(&sources), Some(old.check(&sources)));
}

const CYCLIC_ROOTS: [OpenGroup; 12] = [
    (
        &[
            "fn f(v): v.past(1, 0)\n",
            "state x(1)\n",
            "state h = f(h) + x\n",
        ],
        &[("declaration-cycle", "h"), ("history-target", "v")],
    ),
    (
        &[
            "fn f(v): v.past(1, 0)\n",
            "state x(1)\n",
            "state h = f(x) + h\n",
        ],
        &[("declaration-cycle", "h"), ("history-target", "v")],
    ),
    (
        &[
            "fn f(v): v.past(1, 0)\n",
            "state x(1)\n",
            "state h = f(g) + x\n",
            "state g = h\n",
        ],
        &[("declaration-cycle", "h"), ("history-target", "v")],
    ),
    (
        &["fn f(v): v.past(1, 0)\n", "let h = f(h)\n"],
        &[("declaration-cycle", "h"), ("history-target", "v")],
    ),
    (
        &["fn f(v): v.past(1, 0)\n", "let h = b\n", "let b = f(h)\n"],
        &[("declaration-cycle", "h"), ("history-target", "v")],
    ),
    (
        &[
            "fn f(v): 1 + true\n",
            "state x(1)\n",
            "state h = f(h) + x\n",
        ],
        &[("declaration-cycle", "h"), ("operand-type", "1 + true")],
    ),
    (
        &["let h = (1 + true) + h\n"],
        &[("declaration-cycle", "h"), ("operand-type", "1 + true")],
    ),
    (
        &["let h = [1][5] + h\n"],
        &[("declaration-cycle", "h"), ("index-out-of-range", "5")],
    ),
    (
        &["fn f(v): v\n", "fn g(v): f(1 + true)\n", "let h = g(h)\n"],
        &[("declaration-cycle", "h"), ("operand-type", "1 + true")],
    ),
    (
        &[
            "state x(1)\n",
            "state h(1 + true) = g + x\n",
            "state g = h\n",
        ],
        &[("declaration-cycle", "h"), ("operand-type", "1 + true")],
    ),
    (
        &["fn f(v): v + 1\n", "state x(1)\n", "state h = f(x) + x\n"],
        &[],
    ),
    (
        &[
            "state x(1)\n",
            "state h(1 + 1) = g + x\n",
            "state g = h.past(1)\n",
        ],
        &[],
    ),
];

#[test]
fn a_cyclic_declarations_rule_draws_every_fault_that_holds_whatever_the_cycle_holds_in_every_order_503()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut programs = 0;
    for (items, wanted) in CYCLIC_ROOTS {
        let mut items: Vec<String> = items.iter().map(|item| (*item).to_string()).collect();
        items.push("Clip c:\n    Note/h\n".to_string());
        let mut wanted: Vec<(String, String)> = wanted
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        wanted.sort();
        for text in orders(&items) {
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            let cycle = found
                .iter()
                .find(|diagnostic| diagnostic.code == crate::diagnostics::Code::DeclarationCycle)
                .map(|diagnostic| {
                    text.get(
                        diagnostic.span.range.start as usize..diagnostic.span.range.end as usize,
                    )
                    .unwrap_or_default()
                    .to_string()
                });
            let wanted: Vec<(String, String)> = wanted
                .iter()
                .map(|(code, span)| match (code.as_str(), &cycle) {
                    ("declaration-cycle", Some(first)) => (code.clone(), first.clone()),
                    _ => (code.clone(), span.clone()),
                })
                .collect();
            assert_eq!(spanned(&text, &found), wanted, "{text}");
            assert_eq!(found, old.check(&sources), "{text}");
            programs += 1;
        }
    }
    assert_eq!(programs, 24 * 8 + 120 + 6 + 2 * 2);
}

#[test]
fn a_family_flows_through_a_functions_or_a_patterns_parameter_to_the_attribute_it_reaches_in_every_order_504()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut programs = 0;
    let mut mismatched = 0;
    for argument in ["2bars", "2s", "2", "delay * 2"] {
        for attribute in ["length", "velocity"] {
            let shapes = [
                (
                    "fn hit(v): v\n".to_string(),
                    format!("Clip c:\n    Note | {attribute}=(hit({argument}))\n"),
                ),
                (
                    format!("pattern Hit(default pitch, v):\n    Note/pitch | {attribute}=v\n"),
                    format!("Clip c:\n    Hit/60 | v={argument}\n"),
                ),
            ];
            for (declaration, placement) in shapes {
                let items = vec!["state delay(250ms)\n".to_string(), declaration, placement];
                for text in orders(&items) {
                    let sources = [source(&text)];
                    let found = new
                        .checked_by_new_core(&sources)
                        .unwrap_or_else(|| panic!("the new core takes {text}"));
                    assert_eq!(found, old.check(&sources), "{text}");
                    mismatched += usize::from(
                        found
                            .iter()
                            .any(|drawn| drawn.code == crate::diagnostics::Code::UnitMismatch),
                    );
                    programs += 1;
                }
            }
        }
    }
    assert_eq!(programs, 4 * 2 * 2 * 6);
    assert_eq!(mismatched, (1 + 1 + 1 + 2) * 2 * 6);
}

#[test]
fn a_condition_of_a_destination_beside_a_name_that_names_nothing_draws_its_own_faults() {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for condition in [
        "-flash_on",
        "not -flash_on",
        "(1 + true)",
        "[1][3] > 0",
        "flash_on",
    ] {
        for arms in ["flash else dusk", "dusk else dusk"] {
            let text = format!(
                "state flash_on(false)\nScene dusk:\n    Triggers: night\nScene night:\n    Triggers: if {condition} then {arms}\n"
            );
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
}

#[test]
fn a_family_read_inside_a_placement_for_body_or_a_pattern_use_body_takes_that_bodys_facts_in_every_order_506()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let breaker = "Session main:\n    Includes: t\nTrack t:\n    Piano\n    Starts: c\nClip c:\n    for i in 0..bars:\n        Note/(60 + i) | length=1bars\n    Note/(bars * 12) | length=4bars + bars\n    Note/48 | length=bars\n";
    let mut taken = 0;
    let mut declined = 0;
    let mut programs = 0;
    for tail in ["length=4bars + 1", "length=4bars", "velocity=4bars + 1"] {
        let shapes = [
            (
                String::new(),
                format!("Clip c:\n    for i in 0..bars:\n        Note/(60 + i) | length=1bars\n    Note/48 | {tail}\n"),
                true,
            ),
            (
                "pattern Hit(default pitch, v):\n    Note/pitch | length=v\n".to_string(),
                format!("Clip c:\n    Hit/60 | v=1bars + bars\n    Note/48 | {tail}\n"),
                true,
            ),
            (
                "pattern Hit(default pitch, v):\n    for i in 0..2:\n        Note/(pitch + i) | length=v\n".to_string(),
                format!("Clip c:\n    Hit/60 | v=1bars\n    Hit/62 | v=bars + 2bars\n    Note/48 | {tail}\n"),
                true,
            ),
            (
                String::new(),
                format!("Clip c:\n    Note/60 | length=(for i in 0..bars into a from 0bars: a + 1bars)\n    Note/48 | {tail}\n"),
                false,
            ),
        ];
        for (declaration, placement, landed) in shapes {
            let mut items = vec!["let bars = 4\n".to_string(), placement];
            if !declaration.is_empty() {
                items.push(declaration);
            }
            for text in orders(&items) {
                let sources = [source(&text)];
                programs += 1;
                match new.checked_by_new_core(&sources) {
                    Some(found) => {
                        assert_eq!(found, old.check(&sources), "{text}");
                        taken += 1;
                    }
                    None => {
                        assert!(!landed, "the new core takes {text}");
                        declined += 1;
                    }
                }
            }
        }
    }
    let items = ["let bars = 4\n".to_string(), breaker.to_string()];
    for text in orders(&items) {
        let sources = [source(&text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
        assert!(found.is_empty(), "{text}");
        programs += 1;
        taken += 1;
    }
    assert_eq!(programs, 3 * (2 + 6 + 6 + 2) + 2);
    assert_eq!(taken + declined, programs);
    assert!(taken >= 3 * (2 + 6 + 6) + 2, "{taken}");
}

#[test]
fn a_let_reaching_a_cell_through_a_call_and_read_only_by_a_placement_loop_head_draws_reactive_let_in_every_order()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut programs = 0;
    let mut reactive = 0;
    for (cell, head) in [
        ("state w0([]): i32[..4]\n", "(ch)"),
        ("state w0(3)\n", "0..ch"),
    ] {
        for functions in [
            "fn i0(z): w0\n",
            "fn i1(z): w0\nfn i0(z): i1(z)\n",
            "fn i0(z): [z, z]\n",
            "fn i0(z): z\n",
        ] {
            let items = vec![
                cell.to_string(),
                functions.to_string(),
                "let ch = i0(0)\n".to_string(),
                format!("Clip c:\n    for e in {head}:\n        Note/e\n"),
            ];
            for text in orders(&items) {
                let text = format!(
                    "{text}Session main:\n    Includes: t\nTrack t:\n    Piano\n    Starts: c\n"
                );
                let sources = [source(&text)];
                let found = new
                    .checked_by_new_core(&sources)
                    .unwrap_or_else(|| panic!("the new core takes {text}"));
                assert_eq!(found, old.check(&sources), "{text}");
                reactive += usize::from(
                    found
                        .iter()
                        .any(|drawn| drawn.code == crate::diagnostics::Code::ReactiveLet),
                );
                programs += 1;
            }
        }
    }
    assert_eq!(programs, 2 * 4 * 24);
    assert_eq!(reactive, 2 * 2 * 24);
}

#[test]
fn an_arm_beside_an_arm_in_error_takes_no_expected_type_and_a_refused_float_index_or_bound_is_checked_in_its_own_type()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in [
        "Clip c:\n    Note/(if true then (1 + true) else true)\n",
        "Clip c:\n    Note/(if true then (1 + true) else Wave::Saw)\n",
        "Clip c:\n    Note/(if false then true else (1 + true))\n",
        "Clip c:\n    Note/(if true then (if true then 1 else true) else 2)\n",
        "Clip c:\n    Note/([1, 2][300000000000 % 1.5])\n",
        "Clip c:\n    Note/([1, 2][300000000000 % 1.5] + 1)\n",
        "Clip c:\n    Note/([1, 2][(1 - 300000000000) / 2])\n",
        "Clip c:\n    Note/([1, 2][(1 - 300000000000) / 2..][0])\n",
        "Clip c:\n    Note/([1, 2][300000000000 % 1.5..][0])\n",
        "Clip c:\n    Note/([1, 2][..300000000000 % 1.5][0])\n",
        "Clip c:\n    Note/([1, 2][1.5..][0])\n",
        "let v = [1, 2]\nClip c:\n    Note/(v[300000000000 % 1.5])\n",
        "let x = 300000000000 % 1.5\nClip c:\n    Note/([1, 2][x])\n",
        "let a = [1, 2][300000000000 % 1.5..]\nClip c:\n    Note/(a[0])\n",
        "Clip c:\n    Note/([1, 2][0])\n",
        "Clip c:\n    Note/(if true then 1 else 2)\n",
    ] {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_let_calling_a_function_over_a_chain_of_lets_ending_at_a_cell_draws_reactive_let_as_464_reads_it_507()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for n in [1, 3, 8] {
        let text = super::ledger_tests::lets_calling_functions_over_a_let_chain(n);
        let sources = [source(&text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut wanted: Vec<(String, String)> = (0..=n)
            .map(|at| format!("c{at}"))
            .chain((0..n).map(|at| format!("w{at}")))
            .map(|name| ("reactive-let".to_string(), name))
            .collect();
        wanted.sort();
        assert_eq!(spanned(&text, &found), wanted, "{text}");
        let classes = reviewed::moved(&text, &drawn(&old.check(&sources)), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "464"),
            "{text}"
        );
        let valid = text.replace("let c0 = s\n", "let c0 = 0\n");
        let sources = [source(&valid)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {valid}"));
        assert_eq!(found, old.check(&sources), "{valid}");
        assert_eq!(spanned(&valid, &found), Vec::new(), "{valid}");
    }
}

const REFUSED_CONDITION_AND_BASE_MOVES: [&str; 15] = [
    "Clip c:\n    Note | velocity=(-(if 1.5 then 1.5 else q))\n\nstate q(0): i32\n",
    "Clip c:\n    Note | velocity=((if 1 then q else [1, 2][5]) - (if i32(true) then 1 else 1.5))\n\nstate q(0): u8\n",
    "Clip c:\n    Note/((1 + 300000000000)[if 1 then Wave::Saw else q..])\n\nlet q = [1]\n",
    "Clip c:\n    Note | velocity=((1 + 300000000000)[if 1 then Wave::Saw else q..])\n\nlet q = [1]\n",
    "Clip c:\n    Note/((1 + 300000000000)[if 1 then true else q..])\n\nlet q = [1]\n",
    "Clip c:\n    Note/(if ([1, 2][5] is q) then (Wave::Saw / i32(true)) else (1.5 - 300000000000))\n\nstate q([]): i32[..4]\n",
    "Clip c:\n    Note | velocity=(if ([1, 2][5] is q) then (Wave::Saw / i32(true)) else (1.5 - 300000000000))\n\nstate q([]): i32[..4]\n",
    "Clip c:\n    Note/(if (if 300000000000 then [1, 2] else [1, 2]) then (Wave::Saw / [1, 2]) else ([1, 2] or 1))\n",
    "Clip c:\n    Note | velocity=(if (if 300000000000 then [1, 2] else [1, 2]) then (Wave::Saw / [1, 2]) else ([1, 2] or 1))\n",
    "Clip c:\n    Note | velocity=(if (Wave::Saw or q) then (q % 300000000000) else (1.5 * q))\n\nstate q(0): i32\n",
    "Clip c:\n    Note | velocity=(if (Wave::Saw or q) then (q % 300000000000) else (1.5 * q))\n\nstate q(0): u8\n",
    "Clip c:\n    Note | velocity=(if (Wave::Saw or q) then (q % 300000000000) else (1.5 * q))\n\nstate q(0): i64\n",
    "Clip c:\n    Note | velocity=(if (true or q) then (q % 300000000000) else (1.5 * q))\n\nstate q(0): i32\n",
    "Clip c:\n    Note | velocity=((q[[1, 2]..])[Wave::Saw < 1])\n\nstate q([]): i32[..4]\n",
    "Clip c:\n    Note | velocity=((q[[1, 2]..])[true < 1])\n\nstate q([]): i32[..4]\n",
];

const REFUSED_CONDITION_AND_BASE_TWINS: [&str; 6] = [
    "Clip c:\n    Note/((1 + 300000000000)[0..])\n",
    "state b(false)\nClip c:\n    Note/(if b then (true / i32(true)) else (1.5 - 300000000000))\n",
    "state b(false)\nClip c:\n    Note/(if b then (true / [1, 2]) else ([1, 2] or 1))\n",
    "state b(false)\nClip c:\n    Note | velocity=(if b then (q % 300000000000) else (1.5 * q))\n\nstate q(0): i32\n",
    "Clip c:\n    Note | velocity=((q[0..])[0])\n\nstate q([]): i32[..4]\n",
    "Clip c:\n    Note | velocity=(if (300000000000 and 300000000000) then (q / 1.5) else 2)\n\nstate q(0): i32\n",
];

#[test]
fn faults_in_the_arms_of_a_refused_condition_a_scalar_sliced_base_and_a_cell_read_beside_a_refusal_draw_as_their_repaired_twins_476()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in REFUSED_CONDITION_AND_BASE_MOVES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let classes = reviewed::moved(text, &drawn(&old.check(&sources)), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "476"),
            "{text}"
        );
    }
    for text in REFUSED_CONDITION_AND_BASE_TWINS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_plain_value_in_a_nest_of_functions_draws_one_plain_in_family_per_outermost_call_naming_the_first_longest_chain_508()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for depth in 0..4 {
        for placements in [
            "length,velocity",
            "velocity,length",
            "velocity",
            "velocity,velocity",
            "length",
            "velocity,length,velocity",
        ] {
            let text = super::ledger_tests::family_nest(depth, placements);
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
        }
    }
}

const CELL_KINDS: [&str; 7] = [
    "0): i32",
    "0): u8",
    "0): i64",
    "0.5): f32",
    "0.5): f64",
    "false)",
    "Wave::Sine)",
];

const BESIDE_A_PLACEHOLDER_ARM: [(&str, &[(&str, &str)]); 9] = [
    (
        "if b then (300000000000 + 300000000000) else ([1, 2] / true)",
        &[
            ("does-not-fit", "300000000000 + 300000000000"),
            ("array-operand", "[1, 2] / true"),
        ],
    ),
    (
        "if b then ([1, 2] / true) else (300000000000 + 300000000000)",
        &[
            ("array-operand", "[1, 2] / true"),
            ("does-not-fit", "300000000000 + 300000000000"),
        ],
    ),
    (
        "if true then (300000000000 + 300000000000) else ([1, 2] / true)",
        &[
            ("does-not-fit", "300000000000 + 300000000000"),
            ("array-operand", "[1, 2] / true"),
        ],
    ),
    (
        "if false then (300000000000 + 300000000000) else ([1, 2] / true)",
        &[("array-operand", "[1, 2] / true")],
    ),
    (
        "if b then (if b then (300000000000 + 300000000000) else 1) else ([1, 2] / true)",
        &[
            ("does-not-fit", "300000000000 + 300000000000"),
            ("array-operand", "[1, 2] / true"),
        ],
    ),
    (
        "if b then 300000000000 else ([1, 2] / true)",
        &[
            ("does-not-fit", "300000000000"),
            ("array-operand", "[1, 2] / true"),
        ],
    ),
    (
        "if b then (300000000000 + 300000000000) else i32(true)",
        &[
            ("does-not-fit", "300000000000 + 300000000000"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "if b then (300000000000 + 300000000000) else [1]",
        &[(
            "type-mismatch",
            "if b then (300000000000 + 300000000000) else [1]",
        )],
    ),
    (
        "if b then (300000000000 + 300000000000) else (1 + true)",
        &[("operand-type", "1 + true")],
    ),
];

const REPAIRED_ARMS: [(&str, &[(&str, &str)]); 6] = [
    (
        "if b then (300000000000 + 300000000000) else 2",
        &[("does-not-fit", "300000000000 + 300000000000")],
    ),
    (
        "if b then (300000000000 + 300000000000) else q",
        &[("does-not-fit", "300000000000 + 300000000000")],
    ),
    (
        "if b then [300000000000 + 300000000000][0] else 2",
        &[("does-not-fit", "[300000000000 + 300000000000][0]")],
    ),
    (
        "if b then -(300000000000 + 300000000000) else 2",
        &[("does-not-fit", "-(300000000000 + 300000000000)")],
    ),
    (
        "if b then (if b then (300000000000 + 300000000000) else 1) else 2",
        &[("does-not-fit", "300000000000 + 300000000000")],
    ),
    (
        "if (q or Wave::Saw) then (300000000000 + 300000000000) else 2",
        &[
            ("operand-type", "q or Wave::Saw"),
            ("does-not-fit", "300000000000 + 300000000000"),
        ],
    ),
];

fn assert_in_every_order(
    old: &Compiler,
    new: &Compiler,
    items: &[String],
    wanted: &[(&str, &str)],
) {
    let mut wanted: Vec<(String, String)> = wanted
        .iter()
        .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
        .collect();
    wanted.sort();
    for text in orders(items) {
        let sources = [source(&text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let mut drawn = spanned(&text, &found);
        drawn.sort();
        assert_eq!(drawn, wanted, "{text}");
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

#[test]
fn a_sibling_of_a_placeholder_arm_is_checked_against_the_position_as_its_repaired_twin_is_and_a_flexible_arm_fits_at_its_own_span_in_every_order_509()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for kind in CELL_KINDS {
        for (position, ty) in [("Note/", "i32"), ("Note | velocity=", "f32")] {
            let text = format!(
                "Clip c:\n    {position}(if (q or Wave::Saw) then (300000000000 + 300000000000) else ([1, 2] / true))\n\nstate q({kind}\n"
            );
            let sources = [source(&text)];
            let found = new
                .checked_by_new_core(&sources)
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(found, old.check(&sources), "{text}");
            let fits = found
                .iter()
                .filter(|diagnostic| diagnostic.code == crate::diagnostics::Code::DoesNotFit)
                .all(|diagnostic| diagnostic.summary.ends_with(&format!("`{ty}`")));
            assert!(fits, "{text}");
        }
    }
    for (choice, wanted) in BESIDE_A_PLACEHOLDER_ARM.iter().chain(REPAIRED_ARMS.iter()) {
        let items = [
            format!("Clip c:\n    Note/({choice})\n"),
            "state b(false)\n".to_string(),
            "state q(0): i32\n".to_string(),
        ];
        assert_in_every_order(&old, &new, &items, wanted);
    }
}

type UnifiedArm = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const UNIFIED_ARMS: [UnifiedArm; 7] = [
    (
        "if b then (q % 300000000000) else (1.5 * q)",
        "i32",
        &[("does-not-fit", "300000000000")],
    ),
    (
        "if b then (q % 300000000000) else (1.5 + q)",
        "u8",
        &[("does-not-fit", "300000000000")],
    ),
    (
        "if (Wave::Saw or q) then (q % 300000000000) else (1.5 * q)",
        "i32",
        &[
            ("operand-type", "Wave::Saw or q"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "if (Wave::Saw or q) then (q % 300000000000) else (1.5 * q)",
        "u8",
        &[
            ("operand-type", "Wave::Saw or q"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "if b then 300000000000 else (1.5 * q)",
        "i32",
        &[("does-not-fit", "300000000000")],
    ),
    (
        "if b then (16777217 + 1) else (1.5 * q)",
        "i32",
        &[("does-not-fit", "16777217")],
    ),
    (
        "if b then (q % 30000000000000000000) else (1.5 * q)",
        "i64",
        &[],
    ),
];

#[test]
fn a_choice_its_position_refuses_checks_its_arms_in_their_unified_type_as_the_old_engine_does_in_every_order()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (choice, cell, faults) in UNIFIED_ARMS {
        let mut wanted = vec![("type-mismatch", choice)];
        wanted.extend_from_slice(faults);
        let items = [
            format!("Clip c:\n    Note/({choice})\n"),
            "state b(false)\n".to_string(),
            format!("state q(0): {cell}\n"),
        ];
        assert_in_every_order(&old, &new, &items, &wanted);
    }
    for (choice, _, _) in UNIFIED_ARMS {
        let text = format!("state b(false)\nstate q(0): i32\nClip c:\n    Note/({choice})\n");
        let found = new
            .checked_by_new_core(&[source(&text)])
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let fits = found
            .iter()
            .filter(|diagnostic| diagnostic.code == crate::diagnostics::Code::DoesNotFit)
            .all(|diagnostic| diagnostic.summary.ends_with("`f32`"));
        assert!(fits, "{text}");
    }
}

const CONDITION_CHOICE_MOVES: [&str; 6] = [
    "Clip c:\n    Note | velocity=(if (if [1, 2] then q else Wave::Saw) then (1 + [1, 2][5]) else (q / i32(true)))\n\nstate q(0): i32\n",
    "Clip c:\n    Note | velocity=(if (if [1, 2] then q else Wave::Saw) then (1 + [1, 2][5]) else (q / i32(true)))\n\nstate q(0): u8\n",
    "Clip c:\n    Note | velocity=(if (if [1, 2] then q else Wave::Saw) then (1 + [1, 2][5]) else (q / i32(true)))\n\nstate q(0): i64\n",
    "Clip c:\n    Note | velocity=(if (if [1, 2] then q else Wave::Saw) then 1 else (q / 1.5))\n\nstate q(0): i32\n",
    "Clip c:\n    Note/(if [1] then (q % 300000000000) else (1.5 * q))\n\nstate q(0): i32\n",
    "Clip c:\n    Note/(if [1] then (q % 300000000000) else (1.5 * q))\n\nstate q(0): u8\n",
];

const CONDITION_CHOICE_TWINS: [&str; 4] = [
    "state b(false)\nClip c:\n    Note | velocity=(if (if b then q else Wave::Saw) then (1 + [1, 2][5]) else (q / i32(true)))\n\nstate q(0): i32\n",
    "state b(false)\nClip c:\n    Note | velocity=(if (if b then q else Wave::Saw) then 1 else (q / 1.5))\n\nstate q(0): i32\n",
    "state b(false)\nClip c:\n    Note | velocity=(if (if [1, 2] then b else b) then 1 else (q / 1.5))\n\nstate q(0): i32\n",
    "state b(false)\nClip c:\n    Note/(if b then (q % 300000000000) else (1.5 * q))\n\nstate q(0): i32\n",
];

#[test]
fn a_choice_whose_condition_is_refused_unifies_its_arms_and_reads_its_cells_where_it_stands_as_a_condition_as_its_repaired_twin_does_476()
 {
    let old = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for text in CONDITION_CHOICE_MOVES {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        let classes = reviewed::moved(text, &drawn(&old.check(&sources)), &drawn(&found))
            .unwrap_or_else(|unexplained| panic!("{text}: {unexplained:?}"));
        assert!(
            !classes.is_empty() && classes.iter().all(|class| class.ledger == "476"),
            "{text}"
        );
    }
    for text in CONDITION_CHOICE_TWINS {
        let sources = [source(text)];
        let found = new
            .checked_by_new_core(&sources)
            .unwrap_or_else(|| panic!("the new core takes {text}"));
        assert_eq!(found, old.check(&sources), "{text}");
    }
}

type DefaultUnitProgram = (
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
);

const BARS_DEFAULT: [DefaultUnitProgram; 10] = [
    (
        &["state d(5)", "A a | long=d", "B b | short=d"],
        &[("integer-cell", "d"), ("conflicting-units", "d")],
    ),
    (
        &["state d = 2bars + 1", "A a | long=d", "B b | short=d"],
        &[("conflicting-units", "1")],
    ),
    (
        &[
            "state d(5)",
            "A a | long=(d + 1bars)",
            "B b | short=(d + 1beats)",
        ],
        &[("integer-cell", "d"), ("conflicting-units", "")],
    ),
    (
        &[
            "state d(5)",
            "A a | long=(d + 1bars)",
            "B b | short=(d + 1)",
        ],
        &[("integer-cell", "d")],
    ),
    (&["A a | long=(2bars + 1)"], &[]),
    (&["A a | long=(if true then 1 else 2bars)"], &[]),
    (&["A a | long=[1, 2bars][0]"], &[]),
    (
        &["A a | long=(1e308 + 1bars)"],
        &[("does-not-fit", "1e308")],
    ),
    (
        &[
            "state d(5)",
            "A a | long=d",
            "A e | long=d",
            "B b | short=(d + 0)",
        ],
        &[("integer-cell", "d")],
    ),
    (
        &["state d(5.5)", "B b | short=d", "A a | long=d"],
        &[("conflicting-units", "d")],
    ),
];

const BAR_LENGTHS: [DefaultUnitProgram; 12] = [
    (
        &["state n(2)", "Bar b | bar_len=(n + 1bars) beat_len=n/"],
        &[("expression-syntax", "\n")],
    ),
    (
        &["state n(2)", "Bar b | bar_len=(n + 1bars) beat_len=n+"],
        &[("expression-syntax", "\n")],
    ),
    (
        &["state n(2)", "Bar b | bar_len=(n + 1bars) beat_len=(n / 2)"],
        &[("type-mismatch", "n / 2")],
    ),
    (
        &["state n(2)", "Bar b | bar_len=n beat_len=(n / 2)"],
        &[("type-mismatch", "n / 2")],
    ),
    (
        &["state n(2)", "Bar b | bar_len=n beat_len=(n + 1beats)"],
        &[("conflicting-units", "n + 1beats")],
    ),
    (
        &["Bar b | bar_len=3000000000000000000"],
        &[("does-not-fit", "3000000000000000000")],
    ),
    (&["Bar b | bar_len=2000000000000000000"], &[]),
    (
        &["Bar b | bar_len=(1beats + 3000000000000000000)"],
        &[("does-not-fit", "1beats + 3000000000000000000")],
    ),
    (&["Bar b | bar_len=(2 + 1beats)"], &[]),
    (
        &["state d(5)", "Bar b | bar_len=d beat_len=d"],
        &[("conflicting-units", "d")],
    ),
    (&["Bar b | bar_len=-2305843009213693952"], &[]),
    (&["state q(1): i64", "Bar b | bar_len=(q + 1)"], &[]),
];

const MILLISECONDS: [DefaultUnitProgram; 6] = [
    (&["Env e | attack=5"], &[]),
    (
        &["state q(1): i32", "Env e | attack=(q + 1ms)"],
        &[("integer-cell", "q")],
    ),
    (
        &["state q(1): i32", "Env e | attack=q release=q"],
        &[("integer-cell", "q")],
    ),
    (&["Env e | release=(1e38 + 1s)"], &[]),
    (&["Env e | attack=(2 div 1) release=(5 % 2)"], &[]),
    (
        &[
            "state q(1): i32",
            "Env e | attack=(if q > 0 then 1 else 2ms)",
        ],
        &[],
    ),
];

#[test]
fn a_plain_value_at_an_attribute_whose_default_unit_has_a_scale_other_than_1_reads_in_that_unit_and_two_default_units_conflict_in_every_order()
 {
    let mut programs = 0;
    for (vocabulary, rows) in [
        (
            symphonyscript_fixtures::bounded_vocabulary(),
            BARS_DEFAULT.as_slice(),
        ),
        (
            symphonyscript_fixtures::bar_vocabulary(),
            BAR_LENGTHS.as_slice(),
        ),
        (
            symphonyscript_fixtures::envelope_vocabulary(),
            MILLISECONDS.as_slice(),
        ),
    ] {
        let old = Compiler::new(&vocabulary)
            .expect("the vocabulary compiles")
            .on_the_old_engine();
        let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        for (lines, wanted) in rows {
            let items: Vec<String> = lines.iter().map(|line| format!("{line}\n")).collect();
            for text in orders(&items) {
                let sources = [source(&text)];
                let found = new
                    .checked_by_new_core(&sources)
                    .unwrap_or_else(|| panic!("the new core takes {text}"));
                assert_eq!(found, old.check(&sources), "{text}");
                let drawn: Vec<(String, String)> = spanned(&text, &found)
                    .into_iter()
                    .filter(|(code, _)| code != "count-below-minimum")
                    .collect();
                assert_eq!(drawn.len(), wanted.len(), "{text}");
                for (code, span) in wanted.iter() {
                    assert!(
                        drawn
                            .iter()
                            .any(|(seen, at)| seen == code && (span.is_empty() || at == span)),
                        "{text}"
                    );
                }
                programs += 1;
            }
        }
    }
    assert_eq!(programs, 5 * 6 + 24 + 4 + 5 + 7 * 2 + 3 + 3 * 2);
}

const BODY_UNITS_BAR: &[&[&str]] = &[
    &[
        "fn plus1(x): x + 1",
        "Bar b | bar_len=plus1(1bars) beat_len=plus1(1bars)",
    ],
    &[
        "fn plus1(x): x + 3000000000000000000",
        "Bar b | bar_len=plus1(1beats)",
    ],
    &[
        "fn plus1(x): x + 3000000000000000000",
        "Bar b | beat_len=plus1(1beats)",
    ],
    &[
        "fn plus1(x): x + 2000000000000000000",
        "Bar b | bar_len=plus1(1beats) beat_len=plus1(1beats)",
    ],
    &[
        "fn plus1(x): x + 3000000000000000000",
        "Bar b | bar_len=plus1(1beats) beat_len=plus1(1beats)",
    ],
    &[
        "fn f(x): x + 3000000000000000000",
        "Bar c | beat_len=f(1beats)",
        "Bar b | bar_len=f(1beats)",
    ],
    &["let x = 3000000000000000000", "Bar b | bar_len=x"],
    &["let x = 3000000000000000000", "Bar b | beat_len=x"],
    &["let x = 5", "Bar b | bar_len=x beat_len=x"],
    &["let x = 3000000000000000000 + 1beats", "Bar b | bar_len=x"],
    &["fn f(): 3000000000000000000", "Bar b | bar_len=f()"],
    &[
        "fn f(): 3000000000000000000",
        "Bar b | bar_len=(f() + 1beats)",
    ],
    &["Bar b | bar_len=(i64(3000000000000000000) + 1beats)"],
    &["Bar b | bar_len=[3000000000000000000, 2][0..][0]"],
    &["Bar b | bar_len=[1beats, 3000000000000000000][0..][1]"],
    &["Bar b | bar_len=(for i in [1, 2] into a from 0: a + 1500000000000000000)"],
    &["Bar b | bar_len=(for i in [1, 2] into a from 1beats: a + 1500000000000000000)"],
    &["Bar b | bar_len=(for i in [1, 2] into a from 1beats: a + 1000000000000000000)"],
    &["Bar b | bar_len=(for i in [1beats, 2beats] into a from 0beats: a + i + 1)"],
    &["state n(2)", "Bar b | bar_len=n.past(1)"],
    &["state n(2): i64", "Bar b | bar_len=(n.past(1) + 1beats)"],
    &["fn g(x): x", "Bar b | bar_len=g(3000000000000000000)"],
    &[
        "fn g(x): x",
        "Bar b | bar_len=(g(3000000000000000000) + 1beats)",
    ],
    &["Bar b | bar_len=(if true then 3000000000000000000 else 2)"],
    &["Bar b | bar_len=[3000000000000000000, 2][0]"],
    &["let y = 1beats + 1", "Bar b | bar_len=y beat_len=y"],
    &[
        "let y = 1",
        "Bar b | bar_len=(y + 1beats) beat_len=(y + 1beats)",
    ],
    &[
        "let y = [1bars, 3000000000000000000]",
        "Bar b | bar_len=(y[1])",
    ],
    &[
        "let y = [1bars, 3000000000000000000]",
        "Bar b | bar_len=(y[1] + (if y[1] > 1beats then 1beats else 2beats))",
    ],
    &[
        "fn f(x): x + 1",
        "Bar b | bar_len=(f(1beats) + (if f(1beats) > 2beats then 1beats else 2beats))",
    ],
    &[
        "fn f(x): x + 1",
        "let y = f(1beats)",
        "Bar b | bar_len=y beat_len=y",
    ],
    &[
        "fn f(x): x + 1500000000000000000",
        "Bar b | bar_len=f(f(1beats))",
    ],
    &[
        "let y = 3000000000000000000",
        "Bar b | bar_len=(y + 1beats)",
    ],
    &[
        "let y = 1beats + 1500000000000000000",
        "Bar b | bar_len=(y + y)",
    ],
    &[
        "let y = 1beats + 3000000000000000000",
        "Bar b | beat_len=y",
        "Bar c | bar_len=y",
    ],
    &[
        "let y = 3000000000000000000",
        "Bar b | beat_len=(y + 1beats)",
        "Bar c | bar_len=(y + 1beats)",
    ],
    &[
        "fn f(x): x + 1",
        "fn g(x): f(x) + 1",
        "let y = g(1beats)",
        "Bar b | bar_len=y beat_len=y",
    ],
    &[
        "fn f(x): x + 1",
        "fn g(x): f(x) + f(x)",
        "let y = g(1beats)",
        "Bar b | bar_len=y beat_len=y",
    ],
    &[
        "fn f(x): x + 3000000000000000000",
        "let y = f(1beats)",
        "Bar b | bar_len=y beat_len=y",
        "Bar c | bar_len=f(1beats)",
    ],
    &[
        "fn f(x): for i in [1, 2] into a from x: a + 1",
        "let y = f(1beats)",
        "Bar b | bar_len=y beat_len=y",
    ],
    &[
        "fn f(x): for i in [1, 2] into a from x: a + 3000000000000000000",
        "Bar b | beat_len=f(1beats) bar_len=f(1beats)",
    ],
    &[
        "fn f(a): a[1]",
        "Bar b | bar_len=f([1bars, 3000000000000000000])",
    ],
    &[
        "fn f(a): a[1] + 1beats",
        "Bar b | bar_len=f([1, 3000000000000000000])",
    ],
    &["fn f(): 1", "let y = f()", "Bar b | bar_len=y beat_len=y"],
    &["Bar b | bar_len=i32(3)"],
    &["Bar b | bar_len=(u8(3) + 1beats)"],
];

const BODY_UNITS_BOUNDED: &[&[&str]] = &[
    &[
        "fn plus1(x): x + 1",
        "A a | long=plus1(5bars)",
        "B b | short=plus1(5bars)",
    ],
    &[
        "fn plus1(x): x + 1e308",
        "A a | long=plus1(5bars)",
        "B b | short=plus1(5bars)",
    ],
    &["fn f(): 1e308", "A a | long=f()"],
    &["fn f(): 1e308", "A a | long=(f() + 1beats)"],
    &["let y = 1e308", "A a | long=y"],
    &["let y = 1e308", "A a | long=(y + 1beats)"],
    &["let y = 1e308 + 1beats", "A a | long=y", "B b | short=y"],
    &["let y = 1e308 + 1beats", "A a | long=y"],
    &["fn f(x): x + 1e308", "let y = f(1beats)", "A a | long=y"],
    &[
        "fn f(x): x + 1",
        "let y = f(1beats)",
        "A a | long=y",
        "B b | short=y",
    ],
    &["A a | long=f64(1e308)"],
    &["A a | long=(f64(1e308) + 1beats)"],
    &["A a | long=f32(1e38)"],
    &["A a | long=[1e308, 2.0][0..][0]"],
    &["state q(1.0)", "A a | long=(q.past(1) + 1e308)"],
    &[
        "fn f(x): x + 1",
        "fn g(x): f(x) + f(x)",
        "A a | long=g(1beats)",
        "B b | short=g(1beats)",
    ],
    &["fn f(x): x * 2", "A a | long=(f(1e308) + 1beats)"],
    &["fn f(x): x", "A a | long=f(f(1e308))"],
    &["fn f(x): [x, 1e308]", "A a | long=(f(1beats)[1])"],
];

const BODY_UNITS_ENVELOPE: &[&[&str]] = &[
    &["fn f(x): x + 1", "Env e | attack=f(1s) release=f(1s)"],
    &["let y = 1s + 2", "Env e | attack=y release=y"],
    &["let y = 2", "Env e | attack=(y + 1s) release=(y + 1s)"],
    &["Env e | release=(i32(5) + 1s)"],
    &["state q(1): i32", "Env e | attack=(q.past(1) + 1s)"],
    &["Env e | attack=(for i in [1, 2] into a from 0s: a + 1)"],
    &["fn f(x): x div 2", "Env e | attack=(f(5) + 1s)"],
    &["let y = 5 div 2", "Env e | attack=(y + 1s)"],
    &["fn f(x): (x div 2) + 1s", "Env e | attack=f(5)"],
];

const BODY_UNITS_DECLINED: &[&[&str]] = &[
    &[
        "fn g(): 1bars + 1",
        "let x = g()",
        "Bar b | bar_len=(g(x)) beat_len=x",
    ],
    &[
        "fn g(): 1bars + 1+",
        "let x = g()",
        "Bar b | bar_len=(g()) beat_len=x",
    ],
    &[
        "fn g(): 1bars + 1",
        "let x = g()+",
        "Bar b | bar_len=(g()) beat_len=x",
    ],
    &[
        "fn g(): 1bars + 1",
        "let x = g(x)",
        "Bar b | bar_len=(g()) beat_len=x",
    ],
    &["Bar b | bar_len=(for i in [1beats, 2beats] into a from 3: a + i)"],
    &["Bar b | bar_len=(for i in [1beats, 2beats] into a from 3000000000000000000: a + i)"],
];

#[test]
fn a_plain_value_in_a_body_reads_in_the_default_unit_its_call_reaches_and_a_default_unit_attribute_reads_lets_calls_folds_and_conversions_in_every_order_lb8()
 {
    let mut programs = 0;
    for (vocabulary, rows) in [
        (symphonyscript_fixtures::bar_vocabulary(), BODY_UNITS_BAR),
        (
            symphonyscript_fixtures::bounded_vocabulary(),
            BODY_UNITS_BOUNDED,
        ),
        (
            symphonyscript_fixtures::envelope_vocabulary(),
            BODY_UNITS_ENVELOPE,
        ),
    ] {
        let old = Compiler::new(&vocabulary)
            .expect("the vocabulary compiles")
            .on_the_old_engine();
        let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        for lines in rows {
            let items: Vec<String> = lines.iter().map(|line| format!("{line}\n")).collect();
            for text in orders(&items) {
                let sources = [source(&text)];
                let found = new
                    .checked_by_new_core(&sources)
                    .unwrap_or_else(|| panic!("the new core takes {text}"));
                assert_eq!(found, old.check(&sources), "{text}");
                programs += 1;
            }
        }
    }
    let vocabulary = symphonyscript_fixtures::bar_vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    for lines in BODY_UNITS_DECLINED {
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        assert!(
            new.checked_by_new_core(&[source(&text)]).is_none(),
            "{text}"
        );
    }
    assert_eq!(programs, 281);
}
