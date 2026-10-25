use std::time::Duration;

use symphonyscript_evolve::{
    Checker, DEFAULT_BUDGET, Finding, Guarded, Measured, Program, ProgramSource, Signal, Work,
    dedup_key, differing_codes, guarded, over_memory_bound, scaled, shrunk, superlinear,
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

const LINEAR: &str = "input swell = 0.5
let wobble = swell * 0.8
fn db(x): 10 ** (x / 20)
let base = 48
Clip melody:
    Note/(base + 7)
    Note/62 | velocity=0.7 brightness=wobble
    Plays: refrain
Clip refrain:
    Note/65
    Plays: melody
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
fn a_program_parses_when_lexing_and_parsing_draw_no_diagnostic() {
    let checker = checker(DEFAULT_BUDGET);
    for (name, source, parses) in [
        ("valid", CLEAN, true),
        (
            "semantic errors only",
            "let x = 1\nlet x = 2.5\nlet y = missing + 1\n",
            true,
        ),
        ("syntax error", "Clip a:\n    Note/(1\n", false),
    ] {
        assert_eq!(
            checker.verdict(&program(&[("main.sym", source)])).parses,
            parses,
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
fn files_that_start_with_indented_lines_draw_no_order_signal_for_the_two_file_program() {
    let checker = checker(DEFAULT_BUDGET);
    let program = program(&[("clips.sym", OPEN_CLIP), ("main.sym", INDENTED_START)]);
    assert_eq!(checker.finding(&program, Signal::Order), None);
    assert!(!checker.verdict(&program).signals.contains(&Signal::Order));
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
        (
            vec!["child-not-accepted", "wire-cycle", "wire-endpoint"],
            vec!["wire-cycle", "wire-endpoint"],
            vec!["child-not-accepted"],
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
        "# Vocabulary: music\ninputs\ncommit on_change\n        master.gain = 1.0\n",
        "# Prelude functions used below: scale_degrees, arp, swing\nuse vocabulary music\nlet base_key   = Key\nlet shape      = arp(scale_degrees, steps = 8)\n}\n",
        "# Drum-and-bass style session built from prelude patterns\n        if step % 2 == 1 then amount else 0.0\nsession main\n                pattern = kick_grid\n",
        "let #a = 1\nlet b = #a + \"s\"\nClip c:\n    Note/b\n",
    ] {
        assert_eq!(
            checker.finding(&program(&[("main.sym", text)]), Signal::Rename),
            None,
            "{text}"
        );
    }
}

const CONTINUED: &str = "Clip riff:
    Note as p /60 | velocity=0.8
                    brightness=0.7
                    accent
";

#[test]
fn a_continued_attribute_clause_after_a_declared_name_draws_no_rename_signal() {
    let checker = checker(DEFAULT_BUDGET);
    assert_eq!(
        checker.finding(&program(&[("main.sym", CONTINUED)]), Signal::Rename),
        None
    );
}

#[test]
fn a_wide_name_keeps_its_column_count_in_a_continued_attribute_clause() {
    let checker = checker(DEFAULT_BUDGET);
    let text = format!(
        "Clip riff:\n    Note as \u{65e5} /60 | velocity=0.8\n{}brightness=0.7\n",
        " ".repeat(20)
    );
    assert_eq!(
        checker.finding(&program(&[("main.sym", &text)]), Signal::Rename),
        None
    );
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
    let checker = checker(Duration::ZERO);
    let original = program(&[
        ("clips.sym", OPEN_CLIP_PADDED),
        ("main.sym", INDENTED_START_PADDED),
    ]);
    let Some(finding) = checker.finding(&original, Signal::Slow) else {
        panic!("a zero budget draws no slow finding");
    };
    let smallest = shrunk(&checker, &original, &finding);
    assert!(checker.reproduces(&smallest, &finding));
    assert!(smallest.size() < original.size());
    let key = dedup_key(&finding, &smallest);
    assert!(key.starts_with("slow||"), "{key}");
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

fn measured_work<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Work {
    match guarded(DEFAULT_BUDGET, work) {
        Guarded::Finished { work, .. } => work,
        Guarded::Panicked { message } => panic!("the workload panicked: {message}"),
        Guarded::TimedOut => panic!("the workload timed out"),
    }
}

fn boxed(count: usize) -> Work {
    measured_work(move || (0..count).map(Box::new).collect::<Vec<_>>().len())
}

fn measured(codes_drawn: &[&str], work: Work) -> Measured {
    Measured {
        distinct_codes: codes(codes_drawn),
        work,
    }
}

fn distinct_codes(checker: &Checker, program: &Program) -> Vec<String> {
    let mut drawn: Vec<String> = checker
        .verdict(program)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    drawn.sort();
    drawn.dedup();
    drawn
}

#[test]
fn a_compilation_counts_the_same_allocations_and_peak_bytes_on_every_run() {
    let checker = checker(DEFAULT_BUDGET);
    for text in [CLEAN, FAULTY, LINEAR] {
        let program = program(&[("main.sym", text)]);
        let first = checker.verdict(&program);
        let second = checker.verdict(&program);
        assert!(first.allocations.is_some_and(|count| count > 0), "{text}");
        assert!(first.peak_bytes.is_some_and(|bytes| bytes > 0), "{text}");
        assert_eq!(
            (first.allocations, first.peak_bytes),
            (second.allocations, second.peak_bytes),
            "{text}"
        );
    }
}

#[test]
fn growth_is_superlinear_past_eight_times_above_the_floor_with_the_same_codes() {
    let work = |allocations, peak_bytes| Work {
        allocations,
        peak_bytes,
    };
    let same = ["duplicate-name"];
    let other = ["duplicate-name", "wire-cycle"];
    for (name, once, doubled, quadrupled, expected) in [
        (
            "eight times",
            work(1_000, 100),
            work(2_000, 200),
            work(8_000, 800),
            false,
        ),
        (
            "allocations past eight times",
            work(1_000, 100),
            work(2_000, 200),
            work(8_001, 800),
            true,
        ),
        (
            "peak bytes past eight times",
            work(1_000, 100),
            work(2_000, 200),
            work(8_000, 801),
            true,
        ),
        (
            "under the floor",
            work(999, 100),
            work(1_000_000, 1_000_000),
            work(1_000_000, 1_000_000),
            false,
        ),
    ] {
        assert_eq!(
            superlinear(
                &measured(&same, once),
                &measured(&same, doubled),
                &measured(&same, quadrupled)
            ),
            expected,
            "{name}"
        );
    }
    let once = work(1_000, 100);
    let grown = work(1_000_000, 1_000_000);
    for (name, doubled_codes, quadrupled_codes) in [
        ("new codes at four times", same.as_slice(), other.as_slice()),
        ("new codes at two times", other.as_slice(), same.as_slice()),
    ] {
        assert!(
            !superlinear(
                &measured(&same, once),
                &measured(doubled_codes, grown),
                &measured(quadrupled_codes, grown)
            ),
            "{name}"
        );
    }
}

#[test]
fn a_workload_that_allocates_quadratically_grows_superlinearly_and_a_linear_one_does_not() {
    let linear = |scale: usize| measured(&[], boxed(2_000 * scale));
    let quadratic = |scale: usize| measured(&[], boxed((50 * scale).pow(2)));
    assert!(!superlinear(&linear(1), &linear(2), &linear(4)));
    assert!(superlinear(&quadratic(1), &quadratic(2), &quadratic(4)));
}

#[test]
fn a_program_that_compiles_in_linear_work_draws_no_scaling_signal() {
    let checker = checker(DEFAULT_BUDGET);
    let program = program(&[("main.sym", LINEAR)]);
    let verdict = checker.verdict(&program);
    assert!(verdict.allocations.is_some_and(|count| count >= 1_000));
    assert_eq!(
        distinct_codes(&checker, &scaled(&program, checker.vocabulary(), 4)),
        distinct_codes(&checker, &program)
    );
    assert!(!verdict.signals.contains(&Signal::Scaling));
}

#[test]
fn peak_bytes_past_the_bound_per_source_byte_and_the_floor_draw_the_memory_bound() {
    let floor = 64 * 1024 * 1024;
    for (peak_bytes, source_bytes, bound, expected) in [
        (floor, 1, 50_000, false),
        (floor + 1, 1, 50_000, true),
        (100_000_000, 2_000, 50_000, false),
        (100_000_001, 2_000, 50_000, true),
    ] {
        assert_eq!(
            over_memory_bound(peak_bytes, source_bytes, bound),
            expected,
            "{peak_bytes} bytes for {source_bytes} source bytes at {bound} per byte"
        );
    }
    let workload = measured_work(|| vec![1u8; 80 * 1024 * 1024].len());
    assert!(over_memory_bound(workload.peak_bytes, 1, 50_000));
}
