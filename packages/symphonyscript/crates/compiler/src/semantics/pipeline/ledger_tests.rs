use std::time::{Duration, Instant};

use symphonyscript_fixtures::vocabulary;

use crate::compiler::Compiler;
use crate::counting_allocator::CountingAllocator;
use crate::diagnostics::Diagnostic;
use crate::semantics::counted_walk::CountedWalk;
use crate::source::Source;

struct Run {
    diagnostics: Vec<Diagnostic>,
    time: Duration,
    peak: u64,
    walks: u64,
}

const CORE_WALKS: [CountedWalk; 18] = [
    CountedWalk::BindLookup,
    CountedWalk::CoreExpandRow,
    CountedWalk::SigNode,
    CountedWalk::ValueRun,
    CountedWalk::CheckRow,
    CountedWalk::JoinNode,
    CountedWalk::CursorStep,
    CountedWalk::DenseLayout,
    CountedWalk::CoreUse,
    CountedWalk::CallMemo,
    CountedWalk::SymbolProbe,
    CountedWalk::CoreWireDestination,
    CountedWalk::CheckWireRow,
    CountedWalk::CheckWireCandidate,
    CountedWalk::CheckWireCycle,
    CountedWalk::CorePathStep,
    CountedWalk::CoreLoopIteration,
    CountedWalk::CoreLoopBody,
];

fn walked() -> u64 {
    CORE_WALKS
        .iter()
        .fold(0u64, |total, walk| total.saturating_add(walk.steps()))
}

fn best_run(compiler: &Compiler, text: &str) -> Option<Run> {
    let mut best = run(compiler, text)?;
    for _ in 1..3 {
        let again = run(compiler, text)?;
        best.time = best.time.min(again.time);
    }
    Some(best)
}

fn run(compiler: &Compiler, text: &str) -> Option<Run> {
    let sources = [Source {
        path: "ledger.sym".into(),
        text: text.into(),
    }];
    let before = walked();
    let started = Instant::now();
    let allocated = CountingAllocator::begin();
    let diagnostics = compiler.checked_by_new_core(&sources)?;
    let (_, peak) = CountingAllocator::since(allocated);
    let time = started.elapsed();
    Some(Run {
        diagnostics,
        time,
        peak,
        walks: walked().saturating_sub(before),
    })
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
    let mut codes: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.to_string())
        .filter(|code| code != "no-entry-instance")
        .collect();
    codes.sort();
    codes
}

fn none(_: usize) -> Vec<String> {
    Vec::new()
}

fn one(code: &'static str) -> impl Fn(usize) -> Vec<String> {
    move |_| vec![code.to_string()]
}

fn nested(base: &str, depth: usize, innermost: &str) -> String {
    let mut text = String::from(innermost);
    for _ in 0..depth {
        text = format!("{base}[{text}]");
    }
    text
}

fn undeclared_chain(n: usize) -> String {
    format!("let r = {}\n{}", nested("x", n, "0"), read_of("r"))
}

fn chain_on(start: &str, innermost: &str) -> impl Fn(usize) -> String {
    move |n| {
        format!(
            "let y = [{start}]\nlet r = {}\n{}",
            nested("y", n, innermost),
            read_of("r")
        )
    }
}

fn runtime_chain(n: usize) -> String {
    format!("state y([0])\n{}", read_of(&nested("y", n, "0")))
}

fn let_index_chain(start: &str) -> impl Fn(usize) -> String {
    move |n| {
        let mut text = format!("let y = [0]\nlet a0 = {start}\n");
        for at in 1..=n {
            text.push_str(&format!("let a{at} = y[a{}]\n", at - 1));
        }
        text.push_str(&read_of(&format!("a{n}")));
        text
    }
}

fn cells_over_one_let(n: usize) -> String {
    let terms: Vec<&str> = (0..n).map(|_| "1").collect();
    let mut text = format!("state x(1)\nlet big = {}\n", terms.join(" + "));
    for at in 0..n {
        text.push_str(&format!("state c{at} = big + x\n"));
    }
    text.push_str(&read_of(&format!("c{}", n - 1)));
    text
}

fn doubles_at_most(points: [u64; 3], slack: u64) -> bool {
    let [first, second, third] = points;
    second <= first.saturating_mul(5) / 2 + slack && third <= second.saturating_mul(5) / 2 + slack
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Timing {
    Printed,
    Judged,
}

fn time_failure(name: &str, sizes: [usize; 3], micros: [u64; 3]) -> Option<String> {
    let [smallest, _, largest] = micros;
    let [first, _, last] = sizes;
    let ratio = u64::try_from(last / first.max(1)).unwrap_or(u64::MAX);
    let bound = smallest.saturating_mul(5).saturating_mul(ratio) / 2;
    println!("{name}: {largest} µs at {last} against {smallest} µs at {first}, bound {bound} µs");
    (largest > bound)
        .then(|| format!("{name}: time {micros:?} µs grows past 2.5 times the size ratio {ratio}"))
}

fn ledger_failures(
    name: &str,
    sizes: [usize; 3],
    program: impl Fn(usize) -> String,
    expected: &dyn Fn(usize) -> Vec<String>,
    timing: Timing,
) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    ledger_failures_on(&compiler, name, sizes, program, expected, timing)
}

fn ledger_failures_on(
    compiler: &Compiler,
    name: &str,
    sizes: [usize; 3],
    program: impl Fn(usize) -> String,
    expected: &dyn Fn(usize) -> Vec<String>,
    timing: Timing,
) -> Vec<String> {
    let mut failures = Vec::new();
    let mut peaks = [0u64; 3];
    let mut walks = [0u64; 3];
    let mut micros = [0u64; 3];
    for (at, size) in sizes.into_iter().enumerate() {
        let text = program(size);
        let Some(found) = best_run(compiler, &text) else {
            failures.push(format!("{name} at {size}: the new core declined it"));
            continue;
        };
        let drawn = codes(&found.diagnostics);
        let expected = expected(size);
        if drawn != expected {
            failures.push(format!(
                "{name} at {size}: draws {drawn:?}, expected {expected:?}"
            ));
        }
        let spent = u64::try_from(found.time.as_micros()).unwrap_or(u64::MAX);
        println!(
            "{name} at {size}: {} µs best of three, PeakBytes {}, counted walks {}",
            spent, found.peak, found.walks
        );
        if let Some(slot) = walks.get_mut(at) {
            *slot = found.walks;
        }
        if let Some(slot) = peaks.get_mut(at) {
            *slot = found.peak;
        }
        if let Some(slot) = micros.get_mut(at) {
            *slot = spent;
        }
    }
    if !doubles_at_most(walks, 64) {
        failures.push(format!("{name}: counted walks {walks:?} are not linear"));
    }
    if walks.first().copied().unwrap_or(0) == 0 {
        failures.push(format!("{name}: no counted walk at its smallest size"));
    }
    if !doubles_at_most(peaks, 1 << 20) {
        failures.push(format!("{name}: PeakBytes {peaks:?} is not linear"));
    }
    if timing == Timing::Judged {
        failures.extend(time_failure(name, sizes, micros));
    }
    failures
}

fn assert_none(failures: &[String]) {
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn read_of(value: &str) -> String {
    format!("Clip c:\n    Note/({value})\n")
}

fn fold_sum(n: usize) -> String {
    format!(
        "let xs = for i in 0..{n} into a from []: a + [i]\nlet s = for x in xs into t from 0: t + x\n{}",
        read_of("s div 1000000000")
    )
}

fn let_chain(n: usize) -> String {
    let mut text = String::from("let v0 = [1]\n");
    for at in 1..=n {
        text.push_str(&format!("let v{at} = v{} + [1]\n", at - 1));
    }
    text.push_str(&read_of(&format!("v{n}[{n}]")));
    text
}

fn one_join(n: usize) -> String {
    let terms: Vec<&str> = (0..n).map(|_| "q1").collect();
    format!(
        "let q1 = [1]\nlet q = {}\n{}",
        terms.join(" + "),
        read_of(&format!("q[{}]", n - 1))
    )
}

fn last_fold(n: usize) -> String {
    format!(
        "let xs = for i in 0..{n} into a from [0]: a + [a[^1] + 1]\n{}",
        read_of(&format!("xs[{n}] - {n}"))
    )
}

fn halving_fold(n: usize) -> String {
    format!(
        "let xs = for i in 0..{n} into a from [0]: a + [a[i div 2]]\n{}",
        read_of(&format!("xs[{n}]"))
    )
}

#[test]
fn two_joins_onto_one_older_version_each_keep_their_elements_449() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let text = "let a = [0]\nlet b = a + [1]\nlet d = a + [2]\nClip c:\n    Note/(if b[1] is 1 and d[1] is 2 and a[^1] is 0 then 1 else 1000000000000)\n";
    let found = run(&compiler, text).expect("the new core takes the program");
    assert_eq!(codes(&found.diagnostics), Vec::<String>::new());
}

#[test]
fn a_fold_over_a_range_and_a_fold_over_its_array_draw_nothing_444() {
    assert_none(&ledger_failures(
        "444",
        [200, 400, 800],
        fold_sum,
        &none,
        Timing::Printed,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_a_fold_over_a_range_and_a_fold_over_its_array_cost_time_and_memory_linear_in_the_range_444()
 {
    assert_none(&ledger_failures(
        "444",
        [20_000, 40_000, 80_000],
        fold_sum,
        &none,
        Timing::Judged,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_a_chain_of_joining_lets_costs_time_and_memory_linear_in_the_chain_449() {
    assert_none(&ledger_failures(
        "449",
        [8_000, 16_000, 32_000],
        let_chain,
        &none,
        Timing::Judged,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_one_expression_joining_one_let_many_times_costs_time_and_memory_linear_in_the_joins_449()
{
    assert_none(&ledger_failures(
        "449",
        [25_000, 50_000, 100_000],
        one_join,
        &none,
        Timing::Judged,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_a_fold_reading_its_newest_element_costs_time_and_memory_linear_in_its_range_449() {
    assert_none(&ledger_failures(
        "449",
        [40_000, 80_000, 160_000],
        last_fold,
        &none,
        Timing::Judged,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_a_fold_reading_an_older_element_costs_time_and_memory_linear_in_its_range_449() {
    assert_none(&ledger_failures(
        "449",
        [20_000, 40_000, 80_000],
        halving_fold,
        &none,
        Timing::Judged,
    ));
}

fn unknown_names(n: usize) -> Vec<String> {
    vec!["unknown-name".to_string(); n]
}

fn chains_447(sizes: [usize; 3], undeclared: bool, timing: Timing) -> Vec<String> {
    let mut failures = Vec::new();
    if undeclared {
        failures.extend(ledger_failures(
            "447 undeclared",
            sizes,
            undeclared_chain,
            &unknown_names,
            timing,
        ));
    }
    failures.extend(ledger_failures(
        "447 on [1]",
        sizes,
        chain_on("1", "0"),
        &one("index-out-of-range"),
        timing,
    ));
    failures.extend(ledger_failures(
        "447 on [0] from 1",
        sizes,
        chain_on("0", "1"),
        &one("index-out-of-range"),
        timing,
    ));
    failures.extend(ledger_failures(
        "447 lets from 1",
        sizes,
        let_index_chain("1"),
        &one("index-out-of-range"),
        timing,
    ));
    failures.extend(ledger_failures(
        "447 valid",
        sizes,
        chain_on("0", "0"),
        &none,
        timing,
    ));
    failures.extend(ledger_failures(
        "447 valid lets",
        sizes,
        let_index_chain("0"),
        &none,
        timing,
    ));
    failures.extend(ledger_failures(
        "447 runtime",
        sizes,
        runtime_chain,
        &none,
        timing,
    ));
    failures
}

#[test]
fn an_element_read_whose_index_is_in_error_has_its_elements_type_447() {
    assert_none(&chains_447([25, 50, 100], true, Timing::Printed));
}

#[test]
#[ignore = "stress"]
fn stress_chains_of_element_reads_cost_time_and_memory_linear_in_the_chain_447() {
    assert_none(&chains_447(
        [25_000, 50_000, 100_000],
        false,
        Timing::Judged,
    ));
}

#[test]
fn a_let_reading_a_cell_beside_a_refused_value_draws_both_faults_464() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for value in ["q + true", "(1 + true) + q"] {
        let text = format!("state q(0)\nlet a = {value}\nClip c:\n    Note/a\n");
        let found = run(&compiler, &text).expect("the new core takes the program");
        assert_eq!(
            codes(&found.diagnostics),
            ["operand-type", "reactive-let"],
            "{text}"
        );
    }
    for (value, code) in [
        ("q + i32(true)", "conversion-type"),
        ("q + [1, 2][5]", "index-out-of-range"),
    ] {
        let text = format!("state q(0)\nlet a = {value}\nClip c:\n    Note/a\n");
        let found = run(&compiler, &text).expect("the new core takes the program");
        assert_eq!(codes(&found.diagnostics), [code, "reactive-let"], "{text}");
    }
}

#[test]
fn cells_reading_one_long_let_draw_nothing_467() {
    assert_none(&ledger_failures(
        "467",
        [25, 50, 100],
        cells_over_one_let,
        &none,
        Timing::Printed,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_cells_reading_one_long_let_cost_time_and_memory_linear_in_the_cells_467() {
    assert_none(&ledger_failures(
        "467",
        [5_000, 10_000, 20_000],
        cells_over_one_let,
        &none,
        Timing::Judged,
    ));
}

#[test]
fn only_a_taken_arm_builds_its_values_471() {
    let overrides = symphonyscript_core::LimitOverrides {
        value_elements: Some(10),
        value_elements_warning: Some(10),
        ..symphonyscript_core::LimitOverrides::default()
    };
    let compiler =
        Compiler::with_limits(&vocabulary(), overrides).expect("the vocabulary compiles");
    let big = "let big = [1, 2, 3, 4, 5, 6]\n";
    let untaken = format!(
        "{big}let y = if false then big + big else [1]\n{}",
        read_of("y[0]")
    );
    let found = run(&compiler, &untaken).expect("the new core takes the program");
    assert_eq!(codes(&found.diagnostics), Vec::<String>::new(), "{untaken}");
    let taken = format!(
        "{big}let y = if true then big + big else [1]\n{}",
        read_of("y[0]")
    );
    let found = run(&compiler, &taken).expect("the new core takes the program");
    assert_eq!(codes(&found.diagnostics), ["value-limit"], "{taken}");
}

#[test]
fn a_choice_refused_as_one_value_draws_one_diagnostic_where_it_is_read_411() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let cell = "state s1 = [1.5]\n";
    for (value, code) in [
        ("i32(if true then [1] else s1)", "array-operand"),
        ("[1, 2][(if true then [1] else s1)..][0]", "index-type"),
        ("[1, 2][if true then [1] else s1]", "index-type"),
        ("[1, 2][^(if true then [1] else s1)]", "index-type"),
    ] {
        let text = format!("{cell}{}", read_of(value));
        let found = run(&compiler, &text).expect("the new core takes the program");
        assert_eq!(codes(&found.diagnostics), [code], "{text}");
    }
}

fn calls_of_one_big_let(sites: usize) -> String {
    let elements: Vec<String> = (0..10_000).map(|at| at.to_string()).collect();
    let mut text = format!(
        "let big = [{}]\nfn f(x): x[9999] - x[0]\nClip c:\n",
        elements.join(", ")
    );
    for _ in 0..sites {
        text.push_str("    Note/(f(big) div 10000)\n");
    }
    text
}

fn memo_failures(sizes: [usize; 3], timing: Timing) -> Vec<String> {
    let overrides = symphonyscript_core::LimitOverrides {
        inlined_calls: Some(1_000_000),
        inlined_calls_warning: Some(1_000_000),
        ..symphonyscript_core::LimitOverrides::default()
    };
    let compiler =
        Compiler::with_limits(&vocabulary(), overrides).expect("the vocabulary compiles");
    ledger_failures_on(
        &compiler,
        "the value memo",
        sizes,
        calls_of_one_big_let,
        &none,
        timing,
    )
}

#[test]
fn a_function_called_with_one_big_let_at_many_sites_draws_nothing_425() {
    assert_none(&memo_failures([250, 500, 1_000], Timing::Printed));
}

#[test]
#[ignore = "stress"]
fn stress_a_function_called_with_one_big_let_costs_time_and_memory_linear_in_the_sites_425() {
    assert_none(&memo_failures([25_000, 50_000, 100_000], Timing::Judged));
}

fn figure(n: usize) -> String {
    let mut text = String::from("state x(1)\nClip c:\n");
    for at in 0..n {
        text.push_str(&format!("    Note/(t{at} + a{at}[1] + s{at})\n"));
    }
    for at in 0..n {
        text.push_str(&format!(
            "let a{at} = [{at}, {at} + 1] + [2]\nfn h{at}(v): v + a{at}[0]\nstate s{at} = h{at}(1) + x\nlet t{at} = for j in 0..3 into acc from 0: acc + j * 2\n"
        ));
    }
    text
}

const BUDGETS: [(&str, f64); 4] = [
    ("collect", 0.05),
    ("expand", 0.05),
    ("resolve", 0.25),
    ("infer and check", 0.4),
];

fn stage_run(compiler: &Compiler, text: &str) -> Option<(Run, Vec<(&'static str, Duration)>)> {
    let mut best: Option<(Run, Vec<(&'static str, Duration)>)> = None;
    for _ in 0..3 {
        let again = run(compiler, text)?;
        let stages = super::stages::last();
        best = Some(match best {
            None => (again, stages),
            Some((mut kept, mut kept_stages)) => {
                kept.time = kept.time.min(again.time);
                for (slot, (_, spent)) in kept_stages.iter_mut().zip(stages) {
                    slot.1 = slot.1.min(spent);
                }
                (kept, kept_stages)
            }
        });
    }
    best
}

fn stage_failures(sizes: [usize; 3]) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut peaks = [0u64; 3];
    let mut walks = [0u64; 3];
    let mut micros = [0u64; 3];
    for (at, n) in sizes.into_iter().enumerate() {
        let text = figure(n);
        let Some((found, stages)) = stage_run(&compiler, &text) else {
            failures.push(format!("the figure at {n}: the new core declined it"));
            continue;
        };
        let drawn = codes(&found.diagnostics);
        if !drawn.is_empty() {
            failures.push(format!("the figure at {n}: draws {drawn:?}"));
        }
        let nodes = super::stages::nodes().max(1);
        let mut shown = Vec::new();
        for (stage, spent) in stages {
            let per_node = spent.as_secs_f64() * 1e6 / nodes as f64;
            let budget = BUDGETS
                .iter()
                .find(|(name, _)| *name == stage)
                .map_or(0.0, |(_, budget)| *budget);
            shown.push(format!(
                "{stage} {per_node:.3} µs a node (its release budget {budget})"
            ));
        }
        println!(
            "the figure at {n}, {nodes} nodes, best of three: {}, PeakBytes {}, counted walks {}",
            shown.join(", "),
            found.peak,
            found.walks
        );
        if let Some(slot) = peaks.get_mut(at) {
            *slot = found.peak;
        }
        if let Some(slot) = walks.get_mut(at) {
            *slot = found.walks;
        }
        if let Some(slot) = micros.get_mut(at) {
            *slot = u64::try_from(found.time.as_micros()).unwrap_or(u64::MAX);
        }
    }
    if !doubles_at_most(walks, 64) {
        failures.push(format!(
            "the figure: counted walks {walks:?} are not linear"
        ));
    }
    if walks.first().copied().unwrap_or(0) == 0 {
        failures.push("the figure: no counted walk at its smallest size".to_string());
    }
    if !doubles_at_most(peaks, 1 << 20) {
        failures.push(format!("the figure: PeakBytes {peaks:?} is not linear"));
    }
    failures.extend(time_failure("the figure", sizes, micros));
    failures
}

#[test]
#[ignore = "stress"]
fn stress_the_new_cores_stages_cost_time_linear_in_the_nodes_of_a_non_recursive_figure() {
    assert_none(&stage_failures([2_000, 4_000, 8_000]));
}

fn clips_beside_a_session(n: usize) -> String {
    let mut text = String::from("Session s\n");
    for at in 0..n {
        text.push_str(&format!("Clip c{at}\n"));
    }
    text
}

fn unreachable_clips(n: usize) -> Vec<String> {
    vec!["unreachable-instance".to_string(); n]
}

#[test]
fn clips_no_entry_reaches_draw_one_warning_each() {
    assert_none(&ledger_failures(
        "the unreachable clips",
        [250, 500, 1_000],
        clips_beside_a_session,
        &unreachable_clips,
        Timing::Printed,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_clips_no_entry_reaches_cost_time_and_memory_linear_in_the_clips() {
    assert_none(&ledger_failures(
        "the unreachable clips",
        [40_000, 80_000, 160_000],
        clips_beside_a_session,
        &unreachable_clips,
        Timing::Judged,
    ));
}

fn wire_rings(n: usize) -> String {
    let mut text = String::new();
    for at in 0..n {
        let next = (at + 1) % n;
        let far = (at + 7) % n;
        text.push_str(&format!(
            "Clip clip{at}:\n    Note/1\n    Plays | gain=0.5: clip{next}\n    Starts: clip{far}\n"
        ));
        text.push_str(&format!("Filter filter{at}:\n    Feeds: filter{next}\n"));
    }
    text
}

#[test]
#[ignore = "stress"]
fn stress_wires_to_named_instances_cost_time_and_memory_linear_in_the_wires_and_their_candidates() {
    assert_none(&ledger_failures(
        "wires",
        [5_000, 10_000, 20_000],
        wire_rings,
        &one("wire-cycle"),
        Timing::Judged,
    ));
}

fn path_chains(n: usize) -> String {
    let mut text = String::from("Clip c0 | gain=0.5:\n    Note as h /60\n");
    for at in 1..n {
        let before = at - 1;
        text.push_str(&format!(
            "Clip c{at} | gain=c{before}.gain level=c{before}.h.velocity:\n    Note as h /c{before}.h.pitch\n"
        ));
    }
    text.push_str(
        "let last = c0.level\nClip ring | level=ring.gain gain=ring.level:\n    Note/1\n",
    );
    text
}

#[test]
#[ignore = "stress"]
fn stress_paths_through_named_instances_cost_time_and_memory_linear_in_the_paths_and_their_steps() {
    assert_none(&ledger_failures(
        "paths",
        [5_000, 10_000, 20_000],
        path_chains,
        &one("declaration-cycle"),
        Timing::Judged,
    ));
}

#[test]
fn a_join_of_a_reactive_choice_with_an_array_of_another_type_draws_its_type_mismatch_beside_reactive_let_464()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for (joined, expected) in [
        ("[3]", &["reactive-let"][..]),
        ("[true]", &["reactive-let", "type-mismatch"][..]),
    ] {
        let text = format!(
            "state f(false)\nlet m = (if f then [1, 2] else [3, 4, 5]) + {joined}\nClip c:\n    Note/(m[0])\n"
        );
        let found = run(&compiler, &text).expect("the new core takes the program");
        assert_eq!(codes(&found.diagnostics), expected, "{text}");
    }
}

fn to_chains(n: usize) -> String {
    let mut text = String::new();
    for at in 0..n {
        let next = (at + 1) % n;
        let far = (at + 7) % n;
        text.push_str(&format!(
            "Clip c{at} | gain=0.5:\n    Note as h /60 | velocity=0.25\n    Plays | gain=to.h.velocity transpose=to.h.pitch: c{next}\n    Plays | gain=to.gain: c{far}\n"
        ));
    }
    text
}

#[test]
#[ignore = "stress"]
fn stress_to_paths_cost_time_and_memory_linear_in_the_wires_and_their_paths() {
    assert_none(&ledger_failures(
        "to paths",
        [5_000, 10_000, 20_000],
        to_chains,
        &none,
        Timing::Judged,
    ));
}

fn chosen_rings(n: usize) -> String {
    let mut text = String::from("state look(0)\n");
    for at in 0..n {
        let next = (at + 1) % n;
        let far = (at + 7) % n;
        let back = (at + n - 1) % n;
        text.push_str(&format!(
            "Clip c{at} | gain=0.5:\n    Note as h /60 | velocity=0.25\n    Plays | gain=to.h.velocity transpose=to.h.pitch: [c{next}, c{far}, c{back}][look]\n    Plays | gain=to.gain: if look > 0 then c{far} else (if true then c{next} else c{back})\n"
        ));
    }
    text
}

#[test]
#[ignore = "stress"]
fn stress_chosen_destinations_and_their_to_paths_cost_time_and_memory_linear_in_the_wires_and_their_candidates()
 {
    assert_none(&ledger_failures(
        "chosen destinations",
        [5_000, 10_000, 20_000],
        chosen_rings,
        &none,
        Timing::Judged,
    ));
}

fn placement_for(n: usize) -> String {
    format!("Clip c:\n    for i in 0..{n}:\n        Note/i\n")
}

fn warnings_past_half(n: usize) -> Vec<String> {
    let warnings = usize::from(n >= 500_000) + usize::from(n > 500_000);
    vec!["limit-warning".to_string(); warnings]
}

#[test]
#[ignore = "stress"]
fn stress_a_placement_for_of_a_million_notes_costs_walks_and_peak_bytes_linear_in_the_notes_450() {
    let sizes = [250_000, 500_000, 999_999];
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = ledger_failures_on(
        &compiler,
        "450",
        sizes,
        placement_for,
        &warnings_past_half,
        Timing::Printed,
    );
    let base = run(&compiler, &placement_for(0)).map_or(0, |found| found.peak);
    for size in sizes {
        let Some(found) = run(&compiler, &placement_for(size)) else {
            failures.push(format!("450 at {size}: the new core declined it"));
            continue;
        };
        let items = u64::try_from(size).unwrap_or(u64::MAX);
        let share = found.peak.saturating_sub(base) / items;
        println!("450 at {size}: {share} B an expanded item above the {base} B of no iteration");
        if size == 999_999 && share > 112 {
            failures.push(format!(
                "450 at {size}: {share} B an expanded item, past 112 B"
            ));
        }
    }
    assert_none(&failures);
}
