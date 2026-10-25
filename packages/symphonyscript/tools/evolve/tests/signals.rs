use std::time::Duration;

use symphonyscript_evolve::{
    Checker, DEFAULT_BUDGET, Guarded, Program, ProgramSource, Signal, dedup_key, guarded, shrunk,
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
fn a_diagnostic_that_follows_top_level_order_draws_the_order_signal() {
    let checker = checker(DEFAULT_BUDGET);
    let program = program(&[("main.sym", TWO_SESSIONS)]);
    let verdict = checker.verdict(&program);
    assert_eq!(verdict.signals, vec![Signal::Order]);
    assert!(checker.holds(&program, Signal::Order));
}

#[test]
fn renaming_every_declared_name_keeps_the_diagnostics_of_equal_programs() {
    let checker = checker(DEFAULT_BUDGET);
    for text in [CLEAN, FAULTY, TWO_SESSIONS] {
        assert!(
            !checker.holds(&program(&[("main.sym", text)]), Signal::Rename),
            "{text}"
        );
    }
}

#[test]
fn an_unused_let_at_the_end_keeps_the_diagnostics_of_equal_programs() {
    let checker = checker(DEFAULT_BUDGET);
    for text in [CLEAN, FAULTY, TWO_SESSIONS, "let evolvenoise0 = 1"] {
        assert!(
            !checker.holds(&program(&[("main.sym", text)]), Signal::Noise),
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
        assert!(
            !checker.holds(&program(&[("main.sym", text)]), Signal::Crash),
            "{text}"
        );
    }
}

#[test]
fn shrinking_keeps_the_signal_and_removes_what_it_does_not_need() {
    let checker = checker(DEFAULT_BUDGET);
    let original = program(&[("main.sym", TWO_SESSIONS)]);
    let smallest = shrunk(&checker, &original, Signal::Order);
    assert!(checker.holds(&smallest, Signal::Order));
    assert!(smallest.size() < original.size());
}

#[test]
fn shrinking_a_signal_every_program_shows_leaves_an_empty_program() {
    let checker = checker(Duration::ZERO);
    let smallest = shrunk(&checker, &program(&[("main.sym", CLEAN)]), Signal::Slow);
    assert_eq!(smallest.size(), 0);
}

#[test]
fn the_dedup_key_holds_the_signal_the_sorted_codes_and_the_text_hash() {
    let codes = vec!["unknown-name".to_string(), "duplicate-name".to_string()];
    let first = dedup_key(
        Signal::Order,
        &codes,
        &program(&[("main.sym", "let x = 1\n")]),
    );
    let reversed: Vec<String> = codes.iter().rev().cloned().collect();
    let same = dedup_key(
        Signal::Order,
        &reversed,
        &program(&[("main.sym", "let x = 1\n")]),
    );
    let other = dedup_key(
        Signal::Order,
        &codes,
        &program(&[("main.sym", "let x = 2\n")]),
    );
    assert_eq!(first, same);
    assert!(first.starts_with("order|duplicate-name,unknown-name|"));
    assert_ne!(first, other);
}
