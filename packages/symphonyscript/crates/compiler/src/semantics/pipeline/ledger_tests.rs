use std::time::{Duration, Instant};

use symphonyscript_fixtures::vocabulary;

use crate::compiler::Compiler;
use crate::counting_allocator::CountingAllocator;
use crate::diagnostics::Diagnostic;
use crate::source::Source;

struct Run {
    diagnostics: Vec<Diagnostic>,
    time: Duration,
    peak: u64,
}

fn run(compiler: &Compiler, text: &str) -> Option<Run> {
    let sources = [Source {
        path: "ledger.sym".into(),
        text: text.into(),
    }];
    let started = Instant::now();
    let allocated = CountingAllocator::begin();
    let diagnostics = compiler.checked_by_new_core(&sources)?;
    let (_, peak) = CountingAllocator::since(allocated);
    Some(Run {
        diagnostics,
        time: started.elapsed(),
        peak,
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

fn ledger_failures(
    name: &str,
    sizes: [usize; 3],
    program: impl Fn(usize) -> String,
    expected: &dyn Fn(usize) -> Vec<String>,
) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    ledger_failures_on(&compiler, name, sizes, program, expected)
}

fn ledger_failures_on(
    compiler: &Compiler,
    name: &str,
    sizes: [usize; 3],
    program: impl Fn(usize) -> String,
    expected: &dyn Fn(usize) -> Vec<String>,
) -> Vec<String> {
    let mut failures = Vec::new();
    let mut times = [0u64; 3];
    let mut peaks = [0u64; 3];
    for (at, size) in sizes.into_iter().enumerate() {
        let text = program(size);
        let Some(found) = run(compiler, &text) else {
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
        let micros = u64::try_from(found.time.as_micros()).unwrap_or(u64::MAX);
        println!("{name} at {size}: {} µs, PeakBytes {}", micros, found.peak);
        if let Some(slot) = times.get_mut(at) {
            *slot = micros;
        }
        if let Some(slot) = peaks.get_mut(at) {
            *slot = found.peak;
        }
    }
    if !doubles_at_most(times, 20_000) {
        failures.push(format!("{name}: time {times:?} µs is not linear"));
    }
    if !doubles_at_most(peaks, 1 << 20) {
        failures.push(format!("{name}: PeakBytes {peaks:?} is not linear"));
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
    assert_none(&ledger_failures("444", [200, 400, 800], fold_sum, &none));
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
    ));
}

#[test]
#[ignore = "stress"]
fn stress_a_chain_of_joining_lets_costs_time_and_memory_linear_in_the_chain_449() {
    assert_none(&ledger_failures(
        "449",
        [1_000, 2_000, 4_000],
        let_chain,
        &none,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_one_expression_joining_one_let_many_times_costs_time_and_memory_linear_in_the_joins_449()
{
    assert_none(&ledger_failures(
        "449",
        [5_000, 10_000, 20_000],
        one_join,
        &none,
    ));
}

#[test]
#[ignore = "stress"]
fn stress_a_fold_reading_its_newest_element_costs_time_and_memory_linear_in_its_range_449() {
    assert_none(&ledger_failures(
        "449",
        [20_000, 40_000, 80_000],
        last_fold,
        &none,
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
    ));
}

fn unknown_names(n: usize) -> Vec<String> {
    vec!["unknown-name".to_string(); n]
}

fn chains_447(sizes: [usize; 3], undeclared: bool) -> Vec<String> {
    let mut failures = Vec::new();
    if undeclared {
        failures.extend(ledger_failures(
            "447 undeclared",
            sizes,
            undeclared_chain,
            &unknown_names,
        ));
    }
    failures.extend(ledger_failures(
        "447 on [1]",
        sizes,
        chain_on("1", "0"),
        &one("index-out-of-range"),
    ));
    failures.extend(ledger_failures(
        "447 on [0] from 1",
        sizes,
        chain_on("0", "1"),
        &one("index-out-of-range"),
    ));
    failures.extend(ledger_failures(
        "447 lets from 1",
        sizes,
        let_index_chain("1"),
        &one("index-out-of-range"),
    ));
    failures.extend(ledger_failures(
        "447 valid",
        sizes,
        chain_on("0", "0"),
        &none,
    ));
    failures.extend(ledger_failures(
        "447 valid lets",
        sizes,
        let_index_chain("0"),
        &none,
    ));
    failures.extend(ledger_failures("447 runtime", sizes, runtime_chain, &none));
    failures
}

#[test]
fn an_element_read_whose_index_is_in_error_has_its_elements_type_447() {
    assert_none(&chains_447([25, 50, 100], true));
}

#[test]
#[ignore = "stress"]
fn stress_chains_of_element_reads_cost_time_and_memory_linear_in_the_chain_447() {
    assert_none(&chains_447([2_500, 5_000, 10_000], false));
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
    ));
}

#[test]
#[ignore = "stress"]
fn stress_cells_reading_one_long_let_cost_time_and_memory_linear_in_the_cells_467() {
    assert_none(&ledger_failures(
        "467",
        [2_500, 5_000, 10_000],
        cells_over_one_let,
        &none,
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

fn memo_failures(sizes: [usize; 3]) -> Vec<String> {
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
    )
}

#[test]
fn a_function_called_with_one_big_let_at_many_sites_draws_nothing_425() {
    assert_none(&memo_failures([250, 500, 1_000]));
}

#[test]
#[ignore = "stress"]
fn stress_a_function_called_with_one_big_let_costs_time_and_memory_linear_in_the_sites_425() {
    assert_none(&memo_failures([25_000, 50_000, 100_000]));
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

fn stage_failures(sizes: [usize; 3]) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut figures: Vec<Vec<(&str, f64)>> = Vec::new();
    for n in sizes {
        let text = figure(n);
        let Some(found) = run(&compiler, &text) else {
            failures.push(format!("the figure at {n}: the new core declined it"));
            continue;
        };
        let drawn = codes(&found.diagnostics);
        if !drawn.is_empty() {
            failures.push(format!("the figure at {n}: draws {drawn:?}"));
        }
        let nodes = super::stages::nodes().max(1);
        let mut shown = Vec::new();
        let mut figure = Vec::new();
        for (stage, spent) in super::stages::last() {
            let per_node = spent.as_secs_f64() * 1e6 / nodes as f64;
            let budget = BUDGETS
                .iter()
                .find(|(name, _)| *name == stage)
                .map_or(0.0, |(_, budget)| *budget);
            shown.push(format!(
                "{stage} {per_node:.3} µs a node (its release budget {budget})"
            ));
            figure.push((stage, per_node));
        }
        println!("the figure at {n}, {nodes} nodes: {}", shown.join(", "));
        figures.push(figure);
    }
    if let (Some(first), Some(last)) = (figures.first(), figures.last()) {
        for ((stage, small), (_, large)) in first.iter().zip(last) {
            if *large > small * 1.5 + 0.02 {
                failures.push(format!(
                    "{stage}: {small:.3} µs a node at {} and {large:.3} at {}, not linear",
                    sizes[0], sizes[2]
                ));
            }
        }
    }
    failures
}

#[test]
#[ignore = "stress"]
fn stress_the_new_cores_stages_cost_time_linear_in_the_nodes_of_a_non_recursive_figure() {
    assert_none(&stage_failures([2_000, 4_000, 8_000]));
}
