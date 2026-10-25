use std::time::Duration;

use symphonyscript_evolve::{
    Checker, DEFAULT_BUDGET, Finding, Guarded, Program, ProgramSource, Signal, dedup_key,
    differing_codes, guarded, shrunk,
};

const CLEAN: &str = "input cutoff = 0.5
let wobble = cutoff * 0.8
Session main:
    Includes: lead
Track lead
Clip intro:
    Note/48 Note/52
    Note/62 | velocity=0.7 brightness=wobble
    Plays: verse
Clip verse:
    Note/65
    Plays: intro
";

const FAULTY: &str = "let x = 1
let x = 2.5
let y = missing + 1
fn twice(value) = value * 2
";

const TWO_SESSIONS: &str = "Session a:
    Includes: t
Session b/90
Track t
";

const CYCLE: &str = "let a = b + 1
let b = a * 2
";

const OPEN_CLIP: &str = "Clip intro | gain=0.9:
    Plays: chorus
";

const OPEN_CLIP_PADDED: &str = "input cutoff = 0.5
let wobble = cutoff * 0.8
Clip intro | gain=0.9:
    Plays: chorus
";

const INDENTED_START: &str = "    Track as keys | label=\"keys\":
        Starts: intro
    Includes: rhythm
Track rhythm | label=\"rhythm\":
    Starts: intro
";

const INDENTED_START_PADDED: &str = "    Track as keys | label=\"keys\":
        Starts: intro
    Includes: rhythm
Track rhythm | label=\"rhythm\":
    Starts: intro
let spare = wobble + 3
";

fn program(files: &[(&str, &str)]) -> Program {
    Program {
        id: serde_json::json!("test"),
        sources: files
            .iter()
            .map(|(path, text)| ProgramSource {
                path: (*path).to_string(),
                text: (*text).to_string(),
            })
            .collect(),
    }
}

fn checker(budget: Duration) -> Checker {
    match Checker::new(budget) {
        Ok(checker) => checker,
        Err(failed) => panic!("the fixture vocabulary does not compile: {failed:?}"),
    }
}

fn codes(codes: &[&str]) -> Vec<String> {
    codes.iter().map(|code| (*code).to_string()).collect()
}

#[test]
fn programs_whose_diagnostics_do_not_vary_draw_no_signal() {
    let checker = checker(DEFAULT_BUDGET);
    let split = program(&[("a.sym", "let x = 1\nlet y = x + 1\n"), ("b.sym", CLEAN)]);
    for (name, program) in [
        ("clean", program(&[("main.sym", CLEAN)])),
        ("faulty", program(&[("main.sym", FAULTY)])),
        ("split", split),
        ("empty", program(&[])),
    ] {
        assert_eq!(
            checker.verdict(&program).signals,
            Vec::<Signal>::new(),
            "{name}"
        );
    }
}

#[test]
fn diagnostics_the_spec_ties_to_top_level_order_draw_no_order_signal() {
    let checker = checker(DEFAULT_BUDGET);
    for (name, text) in [("two sessions", TWO_SESSIONS), ("cycle", CYCLE)] {
        let program = program(&[("main.sym", text)]);
        assert_eq!(checker.finding(&program, Signal::Order), None, "{name}");
    }
}

#[test]
fn codes_that_change_when_only_declarations_move_draw_the_order_signal() {
    let checker = checker(DEFAULT_BUDGET);
    let program = program(&[("clips.sym", OPEN_CLIP), ("main.sym", INDENTED_START)]);
    assert_eq!(
        checker.finding(&program, Signal::Order),
        Some(Finding {
            signal: Signal::Order,
            codes: codes(&[
                "child-not-accepted",
                "unexpected-indent",
                "wire-cycle",
                "wire-endpoint",
            ]),
        })
    );
}

#[test]
fn differing_codes_are_the_codes_whose_counts_differ() {
    for (baseline, variant, expected) in [
        (vec!["unknown-name"], vec!["unknown-name"], vec![]),
        (vec![], vec![], vec![]),
        (vec!["unknown-name"], vec![], vec!["unknown-name"]),
        (vec![], vec!["wire-cycle"], vec!["wire-cycle"]),
        (
            vec!["unknown-name", "unknown-name", "wire-cycle"],
            vec!["unknown-name", "wire-cycle"],
            vec!["unknown-name"],
        ),
        (
            vec!["duplicate-name", "unknown-name"],
            vec!["unknown-name", "wire-cycle"],
            vec!["duplicate-name", "wire-cycle"],
        ),
    ] {
        assert_eq!(
            differing_codes(&codes(&baseline), &codes(&variant)),
            codes(&expected),
            "{baseline:?} against {variant:?}"
        );
    }
}

#[test]
fn renaming_every_declared_name_keeps_the_diagnostics_of_equal_programs() {
    let checker = checker(DEFAULT_BUDGET);
    for text in [
        CLEAN,
        FAULTY,
        TWO_SESSIONS,
        "let e\u{301} = 1\nlet b = \u{e9} + 1\n",
    ] {
        assert_eq!(
            checker.finding(&program(&[("main.sym", text)]), Signal::Rename),
            None,
            "{text}"
        );
    }
}

#[test]
fn an_unused_let_at_the_end_keeps_the_diagnostics_of_equal_programs() {
    let checker = checker(DEFAULT_BUDGET);
    for text in [
        CLEAN,
        FAULTY,
        TWO_SESSIONS,
        "let evolvenoise0 = 1",
        "Clip c:\n    Note/1\nlet a = [\n",
        "let a = match 1:\n",
    ] {
        assert_eq!(
            checker.finding(&program(&[("main.sym", text)]), Signal::Noise),
            None,
            "{text}"
        );
    }
}

#[test]
fn a_compilation_past_the_budget_draws_the_slow_signal() {
    let program = program(&[("main.sym", CLEAN)]);
    for (budget, expected) in [
        (Duration::ZERO, vec![Signal::Slow]),
        (DEFAULT_BUDGET, vec![]),
    ] {
        assert_eq!(
            checker(budget).verdict(&program).signals,
            expected,
            "{budget:?}"
        );
    }
}

#[test]
fn a_panic_in_the_guarded_work_is_caught_with_its_message() {
    std::panic::set_hook(Box::new(|_| {}));
    let panicked: Guarded<()> = guarded(DEFAULT_BUDGET, || panic!("lowering lost a cell"));
    let _ = std::panic::take_hook();
    assert_eq!(
        panicked,
        Guarded::Panicked {
            message: "lowering lost a cell".to_string()
        }
    );
}

#[test]
fn guarded_work_that_overruns_its_budget_times_out() {
    let overrun: Guarded<()> = guarded(Duration::from_millis(10), || {
        std::thread::sleep(Duration::from_millis(500));
    });
    assert_eq!(overrun, Guarded::TimedOut);
}

#[test]
fn a_program_without_a_panic_or_an_internal_error_draws_no_crash_signal() {
    let checker = checker(DEFAULT_BUDGET);
    for text in [CLEAN, FAULTY] {
        assert_eq!(
            checker.finding(&program(&[("main.sym", text)]), Signal::Crash),
            None,
            "{text}"
        );
    }
}

#[test]
fn shrinking_ends_at_a_smaller_program_with_the_same_finding_codes() {
    let checker = checker(DEFAULT_BUDGET);
    let original = program(&[
        ("clips.sym", OPEN_CLIP_PADDED),
        ("main.sym", INDENTED_START_PADDED),
    ]);
    let Some(finding) = checker.finding(&original, Signal::Order) else {
        panic!("the padded program draws no order finding");
    };
    let smallest = shrunk(&checker, &original, &finding);
    assert!(checker.reproduces(&smallest, &finding));
    assert!(smallest.size() < original.size());
    let key = dedup_key(&finding, &smallest);
    assert!(
        key.starts_with(&format!("order|{}|", finding.codes.join(","))),
        "{key}"
    );
}

#[test]
fn shrinking_a_signal_every_program_shows_leaves_an_empty_program() {
    let checker = checker(Duration::ZERO);
    let original = program(&[("main.sym", CLEAN)]);
    let Some(finding) = checker.finding(&original, Signal::Slow) else {
        panic!("a zero budget draws no slow finding");
    };
    assert_eq!(shrunk(&checker, &original, &finding).size(), 0);
}

#[test]
fn the_dedup_key_holds_the_signal_the_sorted_codes_and_the_text_hash() {
    let finding = Finding {
        signal: Signal::Order,
        codes: codes(&["unknown-name", "duplicate-name"]),
    };
    let reversed = Finding {
        signal: Signal::Order,
        codes: codes(&["duplicate-name", "unknown-name"]),
    };
    let first = dedup_key(&finding, &program(&[("main.sym", "let x = 1\n")]));
    let same = dedup_key(&reversed, &program(&[("main.sym", "let x = 1\n")]));
    let other = dedup_key(&finding, &program(&[("main.sym", "let x = 2\n")]));
    assert_eq!(first, same);
    assert!(first.starts_with("order|duplicate-name,unknown-name|"));
    assert_ne!(first, other);
}
