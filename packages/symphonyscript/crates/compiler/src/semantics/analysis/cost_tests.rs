use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use symphonyscript_core::Limits;
use symphonyscript_fixtures::vocabulary;

use super::Engine;
use super::addition_tests::linear;
use super::c_array::CArray;
use crate::compiler::Compiler;
use crate::diagnostics::Severity;
use crate::semantics::counted_walk::CountedWalk;
use crate::source::Source;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Construct {
    Let,
    State,
    Inlined,
    Recursive,
    If,
    Match,
    Join,
    Slice,
    Index,
    History,
    ForPlacement,
    ArrayLiteral,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    Chain,
    Ring,
    Nesting,
    Sharing,
    FanIn,
    GrowingArgument,
    NestedCalls,
}

const SHAPES: [Shape; 7] = [
    Shape::Chain,
    Shape::Ring,
    Shape::Nesting,
    Shape::Sharing,
    Shape::FanIn,
    Shape::GrowingArgument,
    Shape::NestedCalls,
];

const HEADER: [&str; 4] = [
    "input note = 60\n",
    "input flag = false\n",
    "input k = 1\n",
    "fn g(v): v + 1\n",
];

const RECURSIVE: &str = "fn r(m): if m > 0 then r(m - 1) else 1\n";

fn step(construct: Construct, value: &str) -> Option<String> {
    Some(match construct {
        Construct::If => format!("(if flag then {value} else 0)"),
        Construct::Join => format!("([{value}] + [1])[0]"),
        Construct::Slice => format!("[{value}, 1][..1][0]"),
        Construct::Index => format!("[{value}, 1][0]"),
        Construct::ArrayLiteral => format!("[[{value}]][0][0]"),
        Construct::Inlined => format!("g({value})"),
        Construct::Recursive => format!("r({value})"),
        Construct::Let
        | Construct::State
        | Construct::Match
        | Construct::History
        | Construct::ForPlacement => return None,
    })
}

fn base(construct: Construct) -> &'static str {
    match construct {
        Construct::Recursive | Construct::ForPlacement => "1",
        Construct::Let
        | Construct::State
        | Construct::Inlined
        | Construct::If
        | Construct::Match
        | Construct::Join
        | Construct::Slice
        | Construct::Index
        | Construct::History
        | Construct::ArrayLiteral => "note",
    }
}

fn reader(value: &str) -> String {
    format!("Clip c:\n    Note/({value})\n")
}

fn sum(names: impl Iterator<Item = String>) -> String {
    names.collect::<Vec<_>>().join(" + ")
}

fn growing(construct: Construct) -> Option<String> {
    let call = "if m > 0 then f";
    Some(match construct {
        Construct::Join | Construct::Recursive => {
            format!("fn f(xs, m): {call}(xs + [1], m - 1) else len(xs)\n")
        }
        Construct::Slice => {
            format!("fn f(xs, m): {call}((xs + [1, 2])[1..], m - 1) else len(xs)\n")
        }
        Construct::Index => format!("fn f(xs, m): {call}(xs + [xs[0]], m - 1) else len(xs)\n"),
        Construct::ArrayLiteral => format!("fn f(xs, m): {call}(xs + [m], m - 1) else len(xs)\n"),
        Construct::If => {
            format!("fn f(xs, m): {call}(if m > 0 then xs + [1] else xs, m - 1) else len(xs)\n")
        }
        Construct::Inlined => {
            format!("fn grow(v): v + [1]\nfn f(xs, m): {call}(grow(xs), m - 1) else len(xs)\n")
        }
        Construct::Let => {
            format!("fn f(xs, m):\n    let ys = xs + [1]\n    {call}(ys, m - 1) else len(ys)\n")
        }
        Construct::Match => {
            "fn f(xs, m):\n    match m:\n        0: len(xs)\n        else: f(xs + [1], m - 1)\n"
                .to_string()
        }
        Construct::State | Construct::History | Construct::ForPlacement => return None,
    })
}

fn nested_function(construct: Construct) -> Option<Vec<String>> {
    let state = "state st = note + 1\n".to_string();
    Some(match construct {
        Construct::Let => vec!["fn h(v):\n    let a = v + 1\n    a\n".to_string()],
        Construct::State => vec![state, "fn h(v): v + st\n".to_string()],
        Construct::Match => {
            vec!["fn h(v):\n    match k:\n        1: v\n        else: 0\n".to_string()]
        }
        Construct::Recursive => Vec::new(),
        Construct::History | Construct::ForPlacement => return None,
        Construct::Inlined
        | Construct::If
        | Construct::Join
        | Construct::Slice
        | Construct::Index
        | Construct::ArrayLiteral => vec![format!("fn h(v): {}\n", step(construct, "v")?)],
    })
}

fn items(construct: Construct, shape: Shape, n: usize) -> Option<Vec<String>> {
    let mut items: Vec<String> = HEADER.iter().map(|item| (*item).to_string()).collect();
    items.push(RECURSIVE.to_string());
    let last = n - 1;
    let start = base(construct);
    match (shape, construct) {
        (Shape::Chain, Construct::Let) => {
            items.push(format!("let v0 = {start} + 1\n"));
            for i in 1..n {
                items.push(format!("let v{i} = v{} + 1\n", i - 1));
            }
            items.push(reader(&format!("v{last}")));
        }
        (Shape::Chain, Construct::State) => {
            items.push(format!("state v0 = {start} + 1\n"));
            for i in 1..n {
                items.push(format!("state v{i} = v{} + 1\n", i - 1));
            }
            items.push(reader(&format!("v{last}")));
        }
        (Shape::Chain, Construct::Match) => {
            items.push(format!("let v0 = {start}\n"));
            for i in 1..n {
                items.push(format!(
                    "let v{i} = match k:\n    1: v{}\n    else: 0\n",
                    i - 1
                ));
            }
            items.push(reader(&format!("v{last}")));
        }
        (Shape::Chain, Construct::History) => {
            items.push(format!("state v0 = {start}{{1, 0}}\n"));
            for i in 1..n {
                items.push(format!("state v{i} = v{}{{1, 0}} + 1\n", i - 1));
            }
            items.push(reader(&format!("v{last}")));
        }
        (Shape::Chain, Construct::ForPlacement) => {
            items.push("let v0 = 1\n".to_string());
            let mut clip = String::from("Clip c:\n");
            for i in 1..n {
                items.push(format!("let v{i} = v{} + 1\n", i - 1));
                clip.push_str(&format!("    for j in [v{i}]:\n        Note/(j)\n"));
            }
            items.push(clip);
        }
        (Shape::Chain, _) => {
            items.push(format!("let v0 = {}\n", step(construct, start)?));
            for i in 1..n {
                items.push(format!(
                    "let v{i} = {}\n",
                    step(construct, &format!("v{}", i - 1))?
                ));
            }
            items.push(reader(&format!("v{last}")));
        }
        (Shape::Ring, Construct::Recursive | Construct::ForPlacement) => return None,
        (Shape::Ring, Construct::Let) => {
            for i in 0..n {
                let previous = (i + last) % n;
                items.push(format!("let l{i} = s{i} + 1\n"));
                items.push(format!("state s{i} = l{previous}{{1, 0}}\n"));
            }
            items.push(reader("s0"));
        }
        (Shape::Ring, Construct::State | Construct::History) => {
            for i in 0..n {
                let previous = (i + last) % n;
                items.push(format!("state s{i} = s{previous}{{1, 0}} + 1\n"));
            }
            items.push(reader("s0"));
        }
        (Shape::Ring, Construct::Match) => {
            for i in 0..n {
                let previous = (i + last) % n;
                items.push(format!(
                    "state s{i} = match k:\n    1: s{previous}{{1, 0}}\n    else: 0\n"
                ));
            }
            items.push(reader("s0"));
        }
        (Shape::Ring, _) => {
            for i in 0..n {
                let previous = (i + last) % n;
                items.push(format!(
                    "state s{i} = {}\n",
                    step(construct, &format!("s{previous}{{1, 0}}"))?
                ));
            }
            items.push(reader("s0"));
        }
        (
            Shape::Nesting,
            Construct::Let
            | Construct::State
            | Construct::Match
            | Construct::Inlined
            | Construct::Recursive,
        ) => return None,
        (Shape::Nesting, Construct::History) => {
            let fallback = format!("{}0{}", "note{1, ".repeat(n), "}".repeat(n));
            items.push(format!("state s = {fallback}\n"));
            items.push(reader("s"));
        }
        (Shape::Nesting, Construct::ForPlacement) => {
            let mut clip = String::from("Clip c:\n");
            for i in 0..n {
                clip.push_str(&format!("{}for j{i} in 0..1:\n", "    ".repeat(i + 1)));
            }
            clip.push_str(&format!("{}Note/(j{last})\n", "    ".repeat(n + 1)));
            items.push(clip);
        }
        (Shape::Nesting, _) => {
            let mut value = start.to_string();
            for _ in 0..n {
                value = step(construct, &value)?;
            }
            items.push(format!("let v = {value}\n"));
            items.push(reader("v"));
        }
        (Shape::Sharing, Construct::Let) => {
            items.push(format!("let v = {start} + 1\n"));
            items.push(format!("Clip c:\n{}", "    Note/(v)\n".repeat(n)));
        }
        (Shape::Sharing, Construct::State) => {
            items.push(format!("state v = {start} + 1\n"));
            items.push(format!("Clip c:\n{}", "    Note/(v)\n".repeat(n)));
        }
        (Shape::Sharing, Construct::Match) => {
            items.push(format!("let v = match k:\n    1: {start}\n    else: 0\n"));
            items.push(format!("Clip c:\n{}", "    Note/(v)\n".repeat(n)));
        }
        (Shape::Sharing, Construct::History) => {
            items.push(format!("state s = {start} + 1\n"));
            for i in 0..n {
                items.push(format!("state t{i} = s{{1, 0}}\n"));
            }
            items.push(reader(&sum((0..n).map(|i| format!("t{i}")))));
        }
        (Shape::Sharing, Construct::ForPlacement) => {
            items.push("let a = [1, 2]\n".to_string());
            items.push(format!(
                "Clip c:\n{}",
                "    for j in a:\n        Note/(j)\n".repeat(n)
            ));
        }
        (Shape::Sharing, _) => {
            items.push(format!("let v = {}\n", step(construct, start)?));
            items.push(format!("Clip c:\n{}", "    Note/(v)\n".repeat(n)));
        }
        (Shape::FanIn, Construct::Let) => {
            for i in 0..n {
                items.push(format!("let v{i} = {start} + {i}\n"));
            }
            items.push(reader(&sum((0..n).map(|i| format!("v{i}")))));
        }
        (Shape::FanIn, Construct::State | Construct::History) => {
            let read = if construct == Construct::History {
                "{1, 0}"
            } else {
                ""
            };
            for i in 0..n {
                items.push(format!("state v{i} = {start} + {i}\n"));
            }
            items.push(format!(
                "state t = {}\n",
                sum((0..n).map(|i| format!("v{i}{read}")))
            ));
            items.push(reader("t"));
        }
        (Shape::FanIn, Construct::Match) => {
            let mut arms = String::new();
            for i in 0..n {
                items.push(format!("let v{i} = {start} + {i}\n"));
                arms.push_str(&format!("    {i}: v{i}\n"));
            }
            items.push(format!("let m = match k:\n{arms}    else: 0\n"));
            items.push(reader("m"));
        }
        (Shape::FanIn, Construct::ForPlacement) => {
            for i in 0..n {
                items.push(format!("let v{i} = {i}\n"));
            }
            let elements = (0..n)
                .map(|i| format!("v{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            items.push(format!(
                "Clip c:\n    for j in [{elements}]:\n        Note/(j)\n"
            ));
        }
        (Shape::FanIn, Construct::ArrayLiteral) => {
            for i in 0..n {
                items.push(format!("let v{i} = {start} + {i}\n"));
            }
            let elements = (0..n)
                .map(|i| format!("v{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            items.push(reader(&format!("[{elements}][k]")));
        }
        (Shape::FanIn, Construct::Join) => {
            for i in 0..n {
                items.push(format!("let v{i} = {start} + {i}\n"));
            }
            let joined = (0..n)
                .map(|i| format!("[v{i}]"))
                .collect::<Vec<_>>()
                .join(" + ");
            items.push(reader(&format!("({joined})[k]")));
        }
        (Shape::FanIn, _) => {
            for i in 0..n {
                items.push(format!("let v{i} = {}\n", step(construct, start)?));
            }
            items.push(reader(&sum((0..n).map(|i| format!("v{i}")))));
        }
        (Shape::GrowingArgument, _) => {
            items.push(growing(construct)?);
            items.push(reader(&format!("f([1], {} - 100000)", 100_000 + n)));
        }
        (Shape::NestedCalls, Construct::Recursive) => {
            let value = format!("{}1{}", "r(".repeat(n), ")".repeat(n));
            items.push(reader(&value));
        }
        (Shape::NestedCalls, _) => {
            items.extend(nested_function(construct)?);
            let value = format!("{}{start}{}", "h(".repeat(n), ")".repeat(n));
            items.push(reader(&value));
        }
    }
    Some(items)
}

struct Measured {
    errors: Vec<String>,
    counts: Vec<(String, u64)>,
}

fn measured(compiler: &Compiler, text: &str) -> Measured {
    let walks_before: Vec<u64> = CountedWalk::ALL.iter().map(|walk| walk.steps()).collect();
    let (built_before, visited_before) = CArray::work();
    let source = Source {
        path: "cost.sym".into(),
        text: text.into(),
    };
    let (_, examined) = compiler.analyzed(&[], |engine: &Engine<'_>| engine.variants.examined());
    let examined = examined.unwrap_or(0);
    let (diagnostics, found) = compiler.analyzed(&[source], |engine: &Engine<'_>| {
        let memo = &engine.memo;
        vec![
            ("capacity_rounds", memo.capacity_rounds),
            ("settlement_visits", memo.settlement_visits),
            ("kind_visits", memo.kind_visits),
            ("capacity_visits", memo.capacity_visits),
            ("reach_work", memo.reach_work),
            ("reach_reports", memo.reach_reports),
            ("layout_visits", memo.layout_visits),
            ("flow_work", memo.flow_work.get()),
            ("use_steps", memo.use_steps.get()),
            ("arm_walks", memo.arm_walks.get()),
            ("query_runs", memo.query_runs),
            ("evaluation_work", memo.evaluation_work),
            ("cycle_work", memo.cycle_work),
            ("read_walk", memo.read_walk),
            ("iterated_elements", memo.iterated_elements),
            (
                "nodes",
                u64::try_from(engine.nodes.len()).unwrap_or(u64::MAX),
            ),
            ("variants_examined", engine.variants.examined() - examined),
        ]
    });
    let (built_after, visited_after) = CArray::work();
    let mut counts: Vec<(String, u64)> = found
        .unwrap_or_default()
        .into_iter()
        .map(|(name, count)| (name.to_string(), count))
        .collect();
    counts.push(("built_elements".into(), built_after - built_before));
    counts.push(("visited_arrays".into(), visited_after - visited_before));
    for (walk, before) in CountedWalk::ALL.iter().zip(walks_before) {
        counts.push((format!("{walk:?}"), walk.steps() - before));
    }
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .map(|diagnostic| format!("{:?} {:?}", diagnostic.code, diagnostic.summary))
        .collect();
    Measured { errors, counts }
}

fn program(construct: Construct, shape: Shape, n: usize, reversed: bool) -> Option<String> {
    let mut items = items(construct, shape, n)?;
    if reversed {
        items.reverse();
    }
    Some(items.concat())
}

const FLOOR: u64 = 6_400;

fn builds_per_level(construct: Construct, shape: Shape) -> bool {
    shape == Shape::GrowingArgument || (shape == Shape::FanIn && construct == Construct::Join)
}

fn indents_per_level(construct: Construct, shape: Shape) -> bool {
    shape == Shape::Nesting && construct == Construct::ForPlacement
}

fn smallest_size(construct: Construct, shape: Shape) -> usize {
    if shape == Shape::GrowingArgument {
        return 100;
    }
    if builds_per_level(construct, shape) || indents_per_level(construct, shape) {
        return 400;
    }
    match (shape, construct) {
        (Shape::Nesting, Construct::Slice) => 2_200,
        (Shape::Nesting, Construct::Join | Construct::Index | Construct::ArrayLiteral) => 3_300,
        _ => 6_500,
    }
}

fn measure(construct: Construct, shape: Shape, n: usize, text: &str) -> u64 {
    let built = match shape {
        Shape::GrowingArgument => (n + 1) * (n + 2) / 2,
        _ if builds_per_level(construct, shape) => n * (n + 1) / 2,
        _ => 0,
    };
    u64::try_from(text.len() + built).unwrap_or(u64::MAX)
}

struct Row {
    construct: Construct,
    shape: Shape,
    reversed: bool,
}

struct Report {
    line: String,
    failures: Vec<String>,
}

fn measured_row(compiler: &Compiler, row: &Row) -> Report {
    let Row {
        construct,
        shape,
        reversed,
    } = *row;
    let smallest = smallest_size(construct, shape);
    let sizes = [smallest, 2 * smallest, 4 * smallest];
    let texts = sizes.map(|n| program(construct, shape, n, reversed).unwrap_or_default());
    let started = std::time::Instant::now();
    let runs = texts.each_ref().map(|text| measured(compiler, text));
    let name = format!(
        "{construct:?} {shape:?} {}",
        if reversed { "reversed" } else { "forward" }
    );
    let mut failures = Vec::new();
    for (run, n) in runs.iter().zip(sizes) {
        if let Some(first) = run.errors.first() {
            failures.push(format!(
                "{name} {n}: {} errors, the first {first}",
                run.errors.len()
            ));
        }
    }
    let mut least = (u64::MAX, "");
    for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
        let points = [0, 1, 2].map(|run| {
            (
                measure(construct, shape, sizes[run], &texts[run]),
                runs[run].counts.get(index).map_or(0, |(_, count)| *count),
            )
        });
        if points[0].1 != points[2].1 && points[0].1 < least.0 {
            least = (points[0].1, counter.as_str());
        }
        if !linear(points) {
            failures.push(format!(
                "{name}: {counter} {:?}",
                points.map(|(_, count)| count)
            ));
        }
    }
    if least.0 < FLOOR
        && !builds_per_level(construct, shape)
        && !indents_per_level(construct, shape)
    {
        failures.push(format!(
            "{name}: the smallest count, {} {}, is below {FLOOR}",
            least.1, least.0
        ));
    }
    let work = runs
        .each_ref()
        .map(|run| run.counts.iter().map(|(_, count)| count).sum::<u64>());
    Report {
        line: format!(
            "{name}: n {sizes:?}, bytes {:?}, smallest count {} {}, all counts {work:?}, {} ms",
            texts.each_ref().map(String::len),
            least.1,
            least.0,
            started.elapsed().as_millis()
        ),
        failures,
    }
}

fn assert_costs_linear(construct: Construct, expected_rows: usize) {
    let limits = Limits {
        nesting_depth: 1_000_000,
        ..Limits::default()
    };
    let mut rows = Vec::new();
    for shape in SHAPES {
        for reversed in [false, true] {
            if items(construct, shape, 1).is_some() {
                rows.push(Row {
                    construct,
                    shape,
                    reversed,
                });
            }
        }
    }
    let next = AtomicUsize::new(0);
    let reports = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism()
        .map_or(4, usize::from)
        .min(rows.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn_scoped(scope, || {
                    let compiler = Compiler::with_limits(&vocabulary(), limits)
                        .expect("the vocabulary compiles");
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(row) = rows.get(index) else {
                            break;
                        };
                        let report = measured_row(&compiler, row);
                        reports
                            .lock()
                            .expect("no worker panics")
                            .push((index, report));
                    }
                })
                .expect("a worker starts");
        }
    });
    let mut reports = reports.into_inner().expect("no worker panics");
    reports.sort_by_key(|(index, _)| *index);
    let mut failures = Vec::new();
    for (_, report) in &reports {
        println!("{}", report.line);
        failures.extend(report.failures.iter().cloned());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(reports.len(), expected_rows);
}

#[test]
fn every_walk_of_check_over_a_let_costs_work_linear_in_the_program_in_every_shape_and_order() {
    assert_costs_linear(Construct::Let, 12);
}

#[test]
fn every_walk_of_check_over_a_state_costs_work_linear_in_the_program_in_every_shape_and_order() {
    assert_costs_linear(Construct::State, 10);
}

#[test]
fn every_walk_of_check_over_an_inlined_function_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Inlined, 12);
}

#[test]
fn every_walk_of_check_over_a_compile_time_recursive_function_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Recursive, 10);
}

#[test]
fn every_walk_of_check_over_an_if_choice_costs_work_linear_in_the_program_in_every_shape_and_order()
{
    assert_costs_linear(Construct::If, 14);
}

#[test]
fn every_walk_of_check_over_a_match_choice_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Match, 12);
}

#[test]
fn every_walk_of_check_over_a_join_costs_work_linear_in_the_program_in_every_shape_and_order() {
    assert_costs_linear(Construct::Join, 14);
}

#[test]
fn every_walk_of_check_over_a_slice_costs_work_linear_in_the_program_in_every_shape_and_order() {
    assert_costs_linear(Construct::Slice, 14);
}

#[test]
fn every_walk_of_check_over_an_index_costs_work_linear_in_the_program_in_every_shape_and_order() {
    assert_costs_linear(Construct::Index, 14);
}

#[test]
fn every_walk_of_check_over_a_history_read_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::History, 10);
}

#[test]
fn every_walk_of_check_over_a_for_placement_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::ForPlacement, 8);
}

#[test]
fn every_walk_of_check_over_an_array_literal_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::ArrayLiteral, 14);
}
