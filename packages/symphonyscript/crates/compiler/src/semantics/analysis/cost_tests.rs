use symphonyscript_core::LimitOverrides;
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
    DotCall,
    Recursive,
    If,
    Match,
    Join,
    Slice,
    Index,
    History,
    Past,
    ForPlacement,
    ArrayLiteral,
    ForExpression,
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
    "state note(60)\n",
    "state flag(false)\n",
    "state k(1)\n",
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
        Construct::ForExpression => format!("(for x in [{value}] into s from 0: s + x)"),
        Construct::Inlined => format!("g({value})"),
        Construct::DotCall => format!("({value}).g()"),
        Construct::Recursive => format!("r({value})"),
        Construct::Let
        | Construct::State
        | Construct::Match
        | Construct::History
        | Construct::Past
        | Construct::ForPlacement => return None,
    })
}

fn base(construct: Construct) -> &'static str {
    match construct {
        Construct::Recursive | Construct::ForPlacement => "1",
        Construct::Let
        | Construct::State
        | Construct::Inlined
        | Construct::DotCall
        | Construct::If
        | Construct::Match
        | Construct::Join
        | Construct::Slice
        | Construct::Index
        | Construct::History
        | Construct::Past
        | Construct::ArrayLiteral
        | Construct::ForExpression => "note",
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
            format!("fn f(xs, m): {call}(xs + [1], m - 1) else xs.size\n")
        }
        Construct::Slice => {
            format!("fn f(xs, m): {call}((xs + [1, 2])[1..], m - 1) else xs.size\n")
        }
        Construct::Index => format!("fn f(xs, m): {call}(xs + [xs[0]], m - 1) else xs.size\n"),
        Construct::ArrayLiteral => format!("fn f(xs, m): {call}(xs + [m], m - 1) else xs.size\n"),
        Construct::ForExpression => format!(
            "fn f(xs, m): {call}((for x in xs into a from [1]: a + [x]), m - 1) else xs.size\n"
        ),
        Construct::If => {
            format!("fn f(xs, m): {call}(if m > 0 then xs + [1] else xs, m - 1) else xs.size\n")
        }
        Construct::Inlined => {
            format!("fn grow(v): v + [1]\nfn f(xs, m): {call}(grow(xs), m - 1) else xs.size\n")
        }
        Construct::DotCall => {
            format!("fn grow(v): v + [1]\nfn f(xs, m): {call}(xs.grow(), m - 1) else xs.size\n")
        }
        Construct::Let => {
            format!("fn f(xs, m):\n    let ys = xs + [1]\n    {call}(ys, m - 1) else ys.size\n")
        }
        Construct::Match => {
            "fn f(xs, m):\n    match m:\n        0: xs.size\n        else: f(xs + [1], m - 1)\n"
                .to_string()
        }
        Construct::State | Construct::History | Construct::Past | Construct::ForPlacement => {
            return None;
        }
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
        Construct::History | Construct::Past | Construct::ForPlacement => return None,
        Construct::Inlined
        | Construct::DotCall
        | Construct::If
        | Construct::Join
        | Construct::Slice
        | Construct::Index
        | Construct::ArrayLiteral
        | Construct::ForExpression => vec![format!("fn h(v): {}\n", step(construct, "v")?)],
    })
}

fn items(construct: Construct, shape: Shape, n: usize, faulty: bool) -> Option<Vec<String>> {
    let mut items: Vec<String> = HEADER.iter().map(|item| (*item).to_string()).collect();
    items.push(RECURSIVE.to_string());
    let last = n - 1;
    let start = if faulty { "q" } else { base(construct) };
    let one = if faulty { "q" } else { "1" };
    let zero = if faulty { "q" } else { "0" };
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
            items.push(format!("state v0 = {start}.past(1, 0)\n"));
            for i in 1..n {
                items.push(format!("state v{i} = v{}.past(1, 0) + 1\n", i - 1));
            }
            items.push(reader(&format!("v{last}")));
        }
        (Shape::Chain, Construct::Past) => {
            items.push(format!("state v0(0) = {start}.past(1)\n"));
            for i in 1..n {
                items.push(format!("state v{i}(0) = v{}.past(1) + 1\n", i - 1));
            }
            items.push(reader(&format!("v{last}")));
        }
        (Shape::Ring, Construct::Past) => {
            for i in 0..n {
                let previous = (i + last) % n;
                items.push(format!("state s{i}(0) = s{previous}.past(1) + {one}\n"));
            }
            items.push(reader("s0"));
        }
        (Shape::Sharing, Construct::Past) => {
            items.push(format!("state s(0) = {start} + 1\n"));
            for i in 0..n {
                items.push(format!("state t{i} = s.past(1)\n"));
            }
            items.push(reader(&sum((0..n).map(|i| format!("t{i}")))));
        }
        (Shape::FanIn, Construct::Past) => {
            for i in 0..n {
                items.push(format!("state v{i}(0) = {start} + {i}\n"));
            }
            items.push(format!(
                "state t = {}\n",
                sum((0..n).map(|i| format!("v{i}.past(1)")))
            ));
            items.push(reader("t"));
        }
        (Shape::Chain, Construct::ForPlacement) => {
            items.push(format!("let v0 = {one}\n"));
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
                items.push(format!("state s{i} = l{previous}.past(1, {zero})\n"));
            }
            items.push(reader("s0"));
        }
        (Shape::Ring, Construct::State | Construct::History) => {
            for i in 0..n {
                let previous = (i + last) % n;
                items.push(format!("state s{i} = s{previous}.past(1, {zero}) + 1\n"));
            }
            items.push(reader("s0"));
        }
        (Shape::Ring, Construct::Match) => {
            for i in 0..n {
                let previous = (i + last) % n;
                items.push(format!(
                    "state s{i} = match k:\n    1: s{previous}.past(1, {zero})\n    else: 0\n"
                ));
            }
            items.push(reader("s0"));
        }
        (Shape::Ring, _) => {
            for i in 0..n {
                let previous = (i + last) % n;
                items.push(format!(
                    "state s{i} = {}\n",
                    step(
                        construct,
                        &format!("s{previous}.past(1, 0){}", if faulty { " + q" } else { "" })
                    )?
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
            | Construct::DotCall
            | Construct::Recursive
            | Construct::Past,
        ) => return None,
        (Shape::Nesting, Construct::History) => {
            let fallback = format!("{}{zero}{}", "note.past(1, ".repeat(n), ")".repeat(n));
            items.push(format!("state s = {fallback}\n"));
            items.push(reader("s"));
        }
        (Shape::Nesting, Construct::ForPlacement) => {
            let mut clip = String::from("Clip c:\n");
            for i in 0..n {
                clip.push_str(&format!("{}for j{i} in 0..{one}:\n", "    ".repeat(i + 1)));
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
                items.push(format!("state t{i} = s.past(1, 0)\n"));
            }
            items.push(reader(&sum((0..n).map(|i| format!("t{i}")))));
        }
        (Shape::Sharing, Construct::ForPlacement) => {
            items.push(format!("let a = [{one}, 2]\n"));
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
                ".past(1, 0)"
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
                items.push(if faulty {
                    format!("let v{i} = q + {i}\n")
                } else {
                    format!("let v{i} = {i}\n")
                });
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
            items.push(reader(&format!("f([{one}], {} - 100000)", 100_000 + n)));
        }
        (Shape::NestedCalls, Construct::Recursive) => {
            let value = format!("{}{one}{}", "r(".repeat(n), ")".repeat(n));
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

fn program(
    construct: Construct,
    shape: Shape,
    n: usize,
    (reversed, faulty): (bool, bool),
) -> Option<String> {
    let mut items = items(construct, shape, n, faulty)?;
    if reversed {
        items.reverse();
    }
    Some(items.concat())
}

const CONSTRUCTS: [Construct; 15] = [
    Construct::Let,
    Construct::State,
    Construct::Inlined,
    Construct::DotCall,
    Construct::Recursive,
    Construct::If,
    Construct::Match,
    Construct::Join,
    Construct::Slice,
    Construct::Index,
    Construct::History,
    Construct::Past,
    Construct::ForPlacement,
    Construct::ArrayLiteral,
    Construct::ForExpression,
];

#[test]
fn every_cost_row_draws_nothing_and_its_erroneous_twin_draws_only_its_unknown_name_303() {
    let limits = LimitOverrides {
        nesting_depth: Some(1_000_000),
        ..LimitOverrides::default()
    };
    let compiler = Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut rows = 0;
    for construct in CONSTRUCTS {
        for shape in SHAPES {
            for faulty in [false, true] {
                let Some(text) = program(construct, shape, 3, (false, faulty)) else {
                    continue;
                };
                rows += 1;
                let errors = measured(&compiler, &text).errors;
                let drawn = if faulty {
                    !errors.is_empty()
                        && errors.iter().all(|error| error.starts_with("UnknownName"))
                } else {
                    errors.is_empty()
                };
                if !drawn {
                    failures.push(format!(
                        "{construct:?} {shape:?} faulty {faulty}: {errors:?}\n{text}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(rows, 178);
}

const FLOOR: u64 = 1_600;

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
        (Shape::Nesting, Construct::Slice) => 550,
        (Shape::Nesting, Construct::Join | Construct::Index | Construct::ArrayLiteral) => 825,
        _ => 1_625,
    }
}

fn elements_built_per_level(construct: Construct, shape: Shape, n: usize) -> usize {
    match shape {
        Shape::GrowingArgument => (n + 1) * (n + 2) / 2,
        _ if builds_per_level(construct, shape) => n * (n + 1) / 2,
        _ => 0,
    }
}

fn program_and_built_elements(construct: Construct, shape: Shape, n: usize, text: &str) -> u64 {
    u64::try_from(text.len() + elements_built_per_level(construct, shape, n)).unwrap_or(u64::MAX)
}

struct Row {
    construct: Construct,
    shape: Shape,
    reversed: bool,
    faulty: bool,
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
        faulty,
    } = *row;
    let smallest = smallest_size(construct, shape);
    let sizes = [smallest, 2 * smallest, 4 * smallest];
    let texts = sizes.map(|n| program(construct, shape, n, (reversed, faulty)).unwrap_or_default());
    let started = std::time::Instant::now();
    let runs = texts.each_ref().map(|text| measured(compiler, text));
    let name = format!(
        "{construct:?} {shape:?} {}{}",
        if reversed { "reversed" } else { "forward" },
        if faulty { " beside a fault (303)" } else { "" }
    );
    let mut failures = Vec::new();
    for (run, n) in runs.iter().zip(sizes) {
        if faulty {
            let unknown = run
                .errors
                .iter()
                .all(|error| error.starts_with("UnknownName"));
            if run.errors.is_empty() || !unknown {
                failures.push(format!(
                    "{name} {n}: draws {:?}, where its one fault is `q`",
                    run.errors
                ));
            }
        } else if let Some(first) = run.errors.first() {
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
                program_and_built_elements(construct, shape, sizes[run], &texts[run]),
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
            "{name}: n {sizes:?}, size {:?}, smallest count {} {}, all counts {work:?}, {} ms",
            [0, 1, 2].map(|run| program_and_built_elements(
                construct,
                shape,
                sizes[run],
                &texts[run]
            )),
            least.1,
            least.0,
            started.elapsed().as_millis()
        ),
        failures,
    }
}

fn assert_costs_linear(construct: Construct, expected_rows: usize) {
    let limits = LimitOverrides {
        nesting_depth: Some(1_000_000),
        ..LimitOverrides::default()
    };
    let mut rows = Vec::new();
    for shape in SHAPES {
        for reversed in [false, true] {
            for faulty in [false, true] {
                if items(construct, shape, 1, faulty).is_some() {
                    rows.push(Row {
                        construct,
                        shape,
                        reversed,
                        faulty,
                    });
                }
            }
        }
    }
    let compiler = Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for row in &rows {
        let report = measured_row(&compiler, row);
        println!("{}", report.line);
        failures.extend(report.failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(rows.len(), expected_rows);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_let_costs_work_linear_in_the_program_in_every_shape_and_order()
{
    assert_costs_linear(Construct::Let, 24);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_state_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::State, 20);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_an_inlined_function_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Inlined, 24);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_dot_call_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::DotCall, 24);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_compile_time_recursive_function_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Recursive, 20);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_an_if_choice_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::If, 28);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_match_choice_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Match, 24);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_join_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Join, 28);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_slice_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Slice, 28);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_an_index_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Index, 28);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_history_read_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::History, 20);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_history_read_without_a_fallback_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::Past, 16);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_for_placement_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::ForPlacement, 16);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_an_array_literal_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::ArrayLiteral, 28);
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_of_check_over_a_for_expression_costs_work_linear_in_the_program_in_every_shape_and_order()
 {
    assert_costs_linear(Construct::ForExpression, 28);
}

fn capacity_program(capacity: usize) -> String {
    format!(
        "state held([]): i32[..{capacity}]\nstate high = for x in held into kept from []: if x > 60 then kept + [x] else kept\nstate probe: i32[..{capacity}] = high\nClip c:\n    Note/(probe[0])\n"
    )
}

fn capacity_failures(sizes: [usize; 3]) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let runs = sizes.map(|capacity| measured(&compiler, &capacity_program(capacity)));
    let mut failures = Vec::new();
    for (run, capacity) in runs.iter().zip(sizes) {
        if let Some(first) = run.errors.first() {
            failures.push(format!("capacity {capacity}: {first}"));
        }
    }
    for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
        let points = [0, 1, 2].map(|run| {
            (
                u64::try_from(sizes[run]).unwrap_or(u64::MAX),
                runs[run].counts.get(index).map_or(0, |(_, count)| *count),
            )
        });
        if !linear(points) {
            failures.push(format!("{counter} {:?}", points.map(|(_, count)| count)));
        }
        if counter == "nodes" && points[0].1 != points[2].1 {
            failures.push(format!(
                "the body is built once, and the nodes number {:?}",
                points.map(|(_, count)| count)
            ));
        }
    }
    failures
}

#[test]
fn a_for_expression_over_a_variable_array_builds_its_body_once_whatever_its_capacity() {
    let failures = capacity_failures([100, 200, 400]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "stress"]
fn stress_a_for_expression_over_a_variable_array_costs_check_work_linear_in_its_capacity() {
    let failures = capacity_failures([2_500, 5_000, 10_000]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn nested_bodies(depth: usize, start: &str) -> String {
    let mut value = String::from("0");
    for level in (0..depth).rev() {
        value = format!(
            "(for x{level} in [1] into a{level} from {start}: a{level} + x{level} + {value})"
        );
    }
    format!("let v = {value}\nClip c:\n    Note/([0][v])\n")
}

fn nesting_failures(sizes: [usize; 3]) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let texts = sizes.map(|depth| nested_bodies(depth, "0"));
    let runs = texts.each_ref().map(|text| measured(&compiler, text));
    let mut failures = Vec::new();
    for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
        let points = [0, 1, 2].map(|run| {
            (
                u64::try_from(texts[run].len()).unwrap_or(u64::MAX),
                runs[run].counts.get(index).map_or(0, |(_, count)| *count),
            )
        });
        if !linear(points) {
            failures.push(format!("{counter} {:?}", points.map(|(_, count)| count)));
        }
    }
    failures
}

#[test]
fn for_expressions_nested_in_their_bodies_resolve_each_body_once() {
    let failures = nesting_failures([50, 100, 200]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "stress"]
fn stress_for_expressions_nested_in_their_bodies_cost_work_linear_in_their_depth() {
    let failures = nesting_failures([1_000, 2_000, 4_000]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Chain {
    ScalarAlias,
    VariantAlias,
    ComparedVariants,
    ArrayAlias,
    EmptyJoin,
    Identity,
    JoinFromLiteral,
    JoinFromNote,
    ReactiveAlias,
    ReactiveArrayAlias,
    Operator,
    VariableGrowth,
    Windowed,
}

const CHAINS: [Chain; 13] = [
    Chain::ScalarAlias,
    Chain::VariantAlias,
    Chain::ComparedVariants,
    Chain::ArrayAlias,
    Chain::EmptyJoin,
    Chain::Identity,
    Chain::JoinFromLiteral,
    Chain::JoinFromNote,
    Chain::ReactiveAlias,
    Chain::ReactiveArrayAlias,
    Chain::Operator,
    Chain::VariableGrowth,
    Chain::Windowed,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ChainRead {
    Pitch,
    Index,
    Len,
}

const CHAIN_READS: [ChainRead; 3] = [ChainRead::Pitch, ChainRead::Index, ChainRead::Len];

impl Chain {
    fn holds_array(self) -> bool {
        !matches!(
            self,
            Chain::ScalarAlias
                | Chain::VariantAlias
                | Chain::ComparedVariants
                | Chain::ReactiveAlias
                | Chain::Operator
        )
    }

    fn grows(self) -> bool {
        matches!(self, Chain::JoinFromLiteral | Chain::JoinFromNote)
    }

    fn first_and_step(self) -> (&'static str, &'static str) {
        match self {
            Chain::ScalarAlias => ("1", "{}"),
            Chain::VariantAlias => ("Saw", "{}"),
            Chain::ComparedVariants => ("Saw", "Sine"),
            Chain::ArrayAlias => ("[0, 1, 2, 3]", "{}"),
            Chain::EmptyJoin => ("[0, 1, 2, 3]", "{} + []"),
            Chain::Identity => ("[0, 1, 2, 3]", "id({})"),
            Chain::JoinFromLiteral => ("[1]", "{} + [1]"),
            Chain::JoinFromNote => ("[note]", "{} + [note]"),
            Chain::ReactiveAlias => ("note", "{}"),
            Chain::ReactiveArrayAlias => ("[note, note, note, note]", "{}"),
            Chain::Operator => ("1", "{} + 1"),
            Chain::VariableGrowth => ("held", "{} + [1]"),
            Chain::Windowed => ("[1]", "({} + [1])[^4..]"),
        }
    }

    fn value(self, read: ChainRead, n: usize) -> Option<usize> {
        match (self, read) {
            (Chain::ScalarAlias, ChainRead::Pitch) => Some(1),
            (Chain::ArrayAlias | Chain::EmptyJoin | Chain::Identity, ChainRead::Index) => Some(3),
            (
                Chain::ArrayAlias | Chain::EmptyJoin | Chain::Identity | Chain::Windowed,
                ChainRead::Len,
            ) => Some(4),
            (Chain::JoinFromLiteral | Chain::Windowed, ChainRead::Index) => Some(1),
            (Chain::JoinFromLiteral | Chain::JoinFromNote, ChainRead::Len) => Some(n),
            (Chain::Operator, ChainRead::Pitch) => Some(n),
            (
                Chain::ScalarAlias
                | Chain::VariantAlias
                | Chain::ComparedVariants
                | Chain::ReactiveAlias
                | Chain::Operator,
                ChainRead::Index | ChainRead::Len,
            )
            | (
                Chain::VariantAlias
                | Chain::ComparedVariants
                | Chain::ArrayAlias
                | Chain::EmptyJoin
                | Chain::Identity
                | Chain::JoinFromLiteral
                | Chain::JoinFromNote
                | Chain::ReactiveAlias
                | Chain::ReactiveArrayAlias
                | Chain::VariableGrowth
                | Chain::Windowed,
                ChainRead::Pitch,
            )
            | (
                Chain::JoinFromNote | Chain::ReactiveArrayAlias | Chain::VariableGrowth,
                ChainRead::Index,
            )
            | (Chain::ReactiveArrayAlias | Chain::VariableGrowth, ChainRead::Len) => None,
        }
    }
}

fn chain_read(read: ChainRead, last: &str) -> String {
    match read {
        ChainRead::Pitch => last.to_string(),
        ChainRead::Index => format!("{last}[3]"),
        ChainRead::Len => format!("({last}).size"),
    }
}

fn chain_items(chain: Chain, read: ChainRead, n: usize, checked: bool) -> Vec<String> {
    let (first, step) = chain.first_and_step();
    let mut items = vec![
        "state note(60)\n".to_string(),
        "state held([1, 2, 3, 4]): i32[..4]\n".to_string(),
        "fn id(x): x\n".to_string(),
        format!("let v0 = {first}\n"),
    ];
    for i in 1..n {
        items.push(format!(
            "let v{i} = {}\n",
            step.replace("{}", &format!("v{}", i - 1))
        ));
    }
    let value = chain_read(read, &format!("v{}", n - 1));
    let pitch = match chain.value(read, n) {
        Some(expected) if checked => format!("[60][{value} - {expected}]"),
        Some(_) | None => value,
    };
    if chain == Chain::ComparedVariants {
        items.push("let t1 = v0 is v1\n".to_string());
        for i in 2..n {
            items.push(format!("let t{i} = (v{} is v{i}) and t{}\n", i - 1, i - 1));
        }
        items.push(format!(
            "Synth s:\n    Osc | wave=v0\nClip c:\n    Note when t{}\n",
            n - 1
        ));
    } else if chain == Chain::VariantAlias {
        items.push(format!("Synth s:\n    Osc | wave={pitch}\n"));
    } else {
        items.push(reader(&pitch));
    }
    items
}

fn chain_program(chain: Chain, read: ChainRead, n: usize, reversed: bool, checked: bool) -> String {
    let mut items = chain_items(chain, read, n, checked);
    if reversed {
        items.reverse();
    }
    items.concat()
}

#[test]
#[ignore = "stress"]
fn stress_every_walk_that_follows_a_chain_of_lets_costs_work_linear_in_the_chain_and_the_elements_it_builds()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut rows = 0;
    for chain in CHAINS {
        for read in CHAIN_READS {
            if chain.holds_array() == (read == ChainRead::Pitch) {
                continue;
            }
            for reversed in [false, true] {
                rows += 1;
                let smallest = if chain.grows() { 100 } else { 1_625 };
                let sizes = [smallest, 2 * smallest, 4 * smallest];
                let name = format!(
                    "{chain:?} {read:?} {}",
                    if reversed { "reversed" } else { "forward" }
                );
                let checked = chain_program(chain, read, smallest, reversed, true);
                let errors = measured(&compiler, &checked).errors;
                if !errors.is_empty() {
                    failures.push(format!("{name}: its value check draws {errors:?}"));
                }
                let texts = sizes.map(|n| chain_program(chain, read, n, reversed, false));
                let runs = texts.each_ref().map(|text| measured(&compiler, text));
                for (run, n) in runs.iter().zip(sizes) {
                    if let Some(first) = run.errors.first() {
                        failures.push(format!("{name} {n}: {first}"));
                    }
                }
                let size = |run: usize| {
                    let built = if chain.grows() {
                        sizes[run] * (sizes[run] + 1) / 2
                    } else {
                        0
                    };
                    u64::try_from(texts[run].len() + built).unwrap_or(u64::MAX)
                };
                for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
                    let points = [0, 1, 2].map(|run| {
                        (
                            size(run),
                            runs[run].counts.get(index).map_or(0, |(_, count)| *count),
                        )
                    });
                    if !linear(points) {
                        failures.push(format!(
                            "{name}: {counter} {:?}",
                            points.map(|(_, count)| count)
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(rows, 42);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ReferenceBody {
    Length,
    FirstGain,
    ChosenGain,
    ReadsGrowing,
    CandidatesGrowingUnderReads,
    ElementsGrowing,
    CandidatesGrowingUnderElements,
    UntakenReadsGrowing,
    CandidatesGrowingUnderUntakenReads,
}

const REFERENCE_BODIES: [ReferenceBody; 9] = [
    ReferenceBody::Length,
    ReferenceBody::FirstGain,
    ReferenceBody::ChosenGain,
    ReferenceBody::ReadsGrowing,
    ReferenceBody::CandidatesGrowingUnderReads,
    ReferenceBody::ElementsGrowing,
    ReferenceBody::CandidatesGrowingUnderElements,
    ReferenceBody::UntakenReadsGrowing,
    ReferenceBody::CandidatesGrowingUnderUntakenReads,
];

const FIXED_COUNT: usize = 8;

fn reference_array_program(body: ReferenceBody, size: usize, call_first: bool) -> String {
    let (k, m) = match body {
        ReferenceBody::ReadsGrowing
        | ReferenceBody::ElementsGrowing
        | ReferenceBody::UntakenReadsGrowing => (FIXED_COUNT, size),
        ReferenceBody::CandidatesGrowingUnderReads
        | ReferenceBody::CandidatesGrowingUnderElements
        | ReferenceBody::CandidatesGrowingUnderUntakenReads => (size, FIXED_COUNT),
        ReferenceBody::Length | ReferenceBody::FirstGain | ReferenceBody::ChosenGain => (size, 1),
    };
    let references = (0..k)
        .map(|index| format!("r{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let reads = vec!["c[i].gain"; m].join(" + ");
    let elements = format!("q([{}], 1).size", vec!["c[i]"; m].join(", "));
    let untaken = format!("(if false then {reads} else 0)");
    let (read, reader) = match body {
        ReferenceBody::ReadsGrowing | ReferenceBody::CandidatesGrowingUnderReads => (
            reads.as_str(),
            format!("Clip k | gain=g([{references}], 0, 3):\n    Note\n"),
        ),
        ReferenceBody::ElementsGrowing | ReferenceBody::CandidatesGrowingUnderElements => (
            elements.as_str(),
            format!("Clip k:\n    Note/(g([{references}], 0, 3))\n"),
        ),
        ReferenceBody::UntakenReadsGrowing | ReferenceBody::CandidatesGrowingUnderUntakenReads => (
            untaken.as_str(),
            format!("Clip k | gain=g([{references}], 0, 3):\n    Note\n"),
        ),
        ReferenceBody::Length => (
            "c.size",
            format!("Clip k:\n    Note/(g([{references}], 0, 3))\n"),
        ),
        ReferenceBody::FirstGain => (
            "c[0].gain",
            format!("Clip k | gain=g([{references}], 0, 3):\n    Note\n"),
        ),
        ReferenceBody::ChosenGain => (
            "c[i].gain",
            format!("Clip k | gain=g([{references}], 0, 3):\n    Note\n"),
        ),
    };
    let mut items = vec![
        reader,
        format!("fn g(c, i, n): if n < 1 then {read} else g(c, i, n - 1)\n"),
        "Session main\n".to_string(),
        "fn q(xs, m): if m < 1 then xs else q(xs, m - 1)\n".to_string(),
    ];
    items.extend((0..k).map(|index| format!("Clip r{index} | gain={index}:\n    Note\n")));
    if !call_first {
        items.reverse();
    }
    items.concat()
}

#[test]
#[ignore = "stress"]
fn stress_a_recursive_call_over_an_array_of_references_costs_work_linear_in_its_elements_in_either_order()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut rows = 0;
    for body in REFERENCE_BODIES {
        for call_first in [true, false] {
            rows += 1;
            let sizes = [200, 400, 800];
            let name = format!(
                "{body:?} {}",
                if call_first {
                    "call first"
                } else {
                    "call last"
                }
            );
            let texts = sizes.map(|size| reference_array_program(body, size, call_first));
            let runs = texts.each_ref().map(|text| measured(&compiler, text));
            for (run, k) in runs.iter().zip(sizes) {
                if let Some(first) = run.errors.first() {
                    failures.push(format!("{name} {k}: {first}"));
                }
            }
            for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
                let points = [0, 1, 2].map(|run| {
                    (
                        u64::try_from(texts[run].len()).unwrap_or(u64::MAX),
                        runs[run].counts.get(index).map_or(0, |(_, count)| *count),
                    )
                });
                if !linear(points) {
                    failures.push(format!(
                        "{name}: {counter} {:?}",
                        points.map(|(_, count)| count)
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(rows, 18);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TemplateWalk {
    PerInstance,
    PerLevelUnderUntakenRead,
}

fn template_walk_program(walk: TemplateWalk, size: usize, call_first: bool) -> String {
    let references = (0..size)
        .map(|index| format!("r{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let read = match walk {
        TemplateWalk::PerInstance => "xs[i].gain",
        TemplateWalk::PerLevelUnderUntakenRead => "(if false then xs[i].gain else 0)",
    };
    let mut items = vec![
        format!(
            "Clip k | gain=h(r0, [{references}], 0, {}):\n    Note\n",
            size - 1
        ),
        format!(
            "fn h(c, xs, i, n): if n < 1 then 0 else {read} + c.gain + h(xs[i], xs, i + 1, n - 1)\n"
        ),
        "Session main\n".to_string(),
    ];
    items.extend((0..size).map(|index| format!("Clip r{index} | gain={}:\n    Note\n", index % 7)));
    if !call_first {
        items.reverse();
    }
    items.concat()
}

#[test]
#[ignore = "stress"]
fn stress_a_recursive_body_with_a_template_per_level_checks_each_frames_heads_in_work_linear_in_its_levels()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut rows = 0;
    for walk in [
        TemplateWalk::PerInstance,
        TemplateWalk::PerLevelUnderUntakenRead,
    ] {
        for call_first in [true, false] {
            rows += 1;
            let sizes = [100, 200, 400];
            let name = format!("{walk:?} call first {call_first}");
            let texts = sizes.map(|size| template_walk_program(walk, size, call_first));
            let runs = texts.each_ref().map(|text| measured(&compiler, text));
            for (run, size) in runs.iter().zip(sizes) {
                if let Some(first) = run.errors.first() {
                    failures.push(format!("{name} {size}: {first}"));
                }
            }
            for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
                let checks = HEAD_CHECKS
                    .iter()
                    .any(|walk| format!("{walk:?}") == *counter);
                let points = [0, 1, 2].map(|run| {
                    let size = sizes[run];
                    let measure = if checks {
                        size
                    } else {
                        texts[run].len() + size * size
                    };
                    (
                        u64::try_from(measure).unwrap_or(u64::MAX),
                        runs[run].counts.get(index).map_or(0, |(_, count)| *count),
                    )
                });
                if !linear(points) {
                    failures.push(format!(
                        "{name}: {counter} {:?}",
                        points.map(|(_, count)| count)
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(rows, 4);
}

const HEAD_CHECKS: [CountedWalk; 3] = [
    CountedWalk::RecursiveFrameHeads,
    CountedWalk::RecursiveBeginChecks,
    CountedWalk::RecursiveCheckedHeads,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UntakenWork {
    OpenKey,
    Descending,
    Starts,
    Candidates,
}

const UNTAKEN_WORK: [UntakenWork; 4] = [
    UntakenWork::OpenKey,
    UntakenWork::Descending,
    UntakenWork::Starts,
    UntakenWork::Candidates,
];

impl UntakenWork {
    fn sizes(self) -> [usize; 3] {
        match self {
            UntakenWork::OpenKey | UntakenWork::Descending => [100, 200, 400],
            UntakenWork::Starts => [25, 50, 100],
            UntakenWork::Candidates => [10, 20, 40],
        }
    }

    fn program(self, size: usize, reversed: bool) -> String {
        let reader = |levels: usize, total: usize| {
            format!("Clip k:\n    Note/([60][h([verse, other], {levels}) - {total}])\n")
        };
        let mut items = vec![
            "Clip verse | gain=2:\n    Note\n".to_string(),
            "Clip other | gain=3:\n    Note\n".to_string(),
        ];
        match self {
            UntakenWork::OpenKey => items.extend([
                format!(
                    "fn g(xs, m): if m < 1 then (if h(xs, {size}) > 0 then xs[0] else xs[1]) else g(xs, m - 1)\n"
                ),
                "fn h(xs, n): if n < 1 then 0 else i32(xs[0].gain) + h(xs, n - 1) + (if false then i32(g(xs, n).gain) else 0)\n".to_string(),
                reader(size, 2 * size),
            ]),
            UntakenWork::Descending => items.extend([
                "fn g(xs, m): if m < 1 then xs[0] else g(xs, m + 1)\n".to_string(),
                "fn h(xs, n): if n < 1 then 0 else i32(xs[0].gain) + h(xs, n - 1) + (if false then i32(g(xs, n).gain) else 0)\n".to_string(),
                reader(size, 2 * size),
            ]),
            UntakenWork::Starts => {
                let starts: String = (0..size)
                    .map(|offset| format!(" + (if false then i32(g(xs, n + {}).gain) else 0)", 100 + offset))
                    .collect();
                items.extend([
                    "fn g(xs, m): if m < 1 then (if h(xs, 100) > 0 then xs[0] else xs[1]) else g(xs, m - 1)\n".to_string(),
                    format!("fn h(xs, n): if n < 1 then 0 else i32(xs[0].gain) + h(xs, n - 1){starts}\n"),
                    reader(100, 200),
                ]);
            }
            UntakenWork::Candidates => {
                let references = (0..size)
                    .map(|index| format!("r{index}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                items.extend((0..size).map(|index| format!("Clip r{index} | gain=2:\n    Note\n")));
                items.extend([
                    "fn h(xs, n): if n < 1 then 0 else 2 + h(xs, n - 1) + (if false then i32(xs[n % xs.size].gain) else 0)\n".to_string(),
                    format!("Clip k:\n    Note/([60][h([{references}], 20) - 40])\n"),
                ]);
            }
        }
        if reversed {
            items.reverse();
        }
        format!("Session main\n{}", items.concat())
    }
}

fn untaken_measured(compiler: &Compiler, text: &str) -> Measured {
    let walks_before: Vec<u64> = CountedWalk::ALL.iter().map(|walk| walk.steps()).collect();
    let source = Source {
        path: "cost.sym".into(),
        text: text.into(),
    };
    let (diagnostics, found) = compiler.analyzed(&[source], |engine: &Engine<'_>| {
        vec![
            ("calls", engine.call_count),
            ("evaluation_work", engine.memo.evaluation_work),
            ("query_runs", engine.memo.query_runs),
            (
                "nodes",
                u64::try_from(engine.nodes.len()).unwrap_or(u64::MAX),
            ),
        ]
    });
    let mut counts: Vec<(String, u64)> = found
        .unwrap_or_default()
        .into_iter()
        .map(|(name, count)| (name.to_string(), count))
        .collect();
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

fn untaken_work_failures(divisor: usize, depth: Option<u64>) -> Vec<String> {
    let limits = LimitOverrides {
        nesting_depth: depth,
        nesting_depth_warning: depth,
        ..LimitOverrides::default()
    };
    let compiler = Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut rows = 0;
    for work in UNTAKEN_WORK {
        for reversed in [false, true] {
            rows += 1;
            let sizes = work.sizes().map(|size| size / divisor);
            let name = format!("{work:?} reversed {reversed}");
            let texts = sizes.map(|size| work.program(size, reversed));
            let runs = texts
                .each_ref()
                .map(|text| untaken_measured(&compiler, text));
            for (run, size) in runs.iter().zip(sizes) {
                let deep = run
                    .errors
                    .iter()
                    .filter(|error| error.starts_with("RecursionDepth"))
                    .count();
                let passes = work == UntakenWork::Descending;
                if deep != run.errors.len() || (deep > 0) != passes {
                    failures.push(format!("{name} {size}: {:?}", run.errors));
                }
            }
            for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
                let against = |measure: [usize; 3]| {
                    [0, 1, 2].map(|run| {
                        (
                            u64::try_from(measure[run]).unwrap_or(u64::MAX),
                            runs[run].counts.get(index).map_or(0, |(_, count)| *count),
                        )
                    })
                };
                let by_size = against(sizes);
                let by_text = against(texts.each_ref().map(String::len));
                let grows = by_size[2].1 > by_size[0].1 + 2;
                if grows && !linear(by_size) && !linear(by_text) {
                    failures.push(format!(
                        "{name}: {counter} {:?}",
                        by_size.map(|(_, count)| count)
                    ));
                }
            }
        }
    }
    assert_eq!(rows, 8);
    failures
}

#[test]
fn an_untaken_starts_evaluation_costs_work_linear_in_the_distinct_calls_plus_the_frames_starts() {
    let failures = untaken_work_failures(5, Some(128));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "stress"]
fn stress_an_untaken_starts_evaluation_costs_work_linear_in_the_distinct_calls_plus_the_frames_starts_in_either_order()
 {
    let failures = untaken_work_failures(1, None);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UnknownNames {
    OneBody,
    Lets,
    Repeated,
    Restored,
    Valid,
}

const UNKNOWN_NAMES: [UnknownNames; 5] = [
    UnknownNames::OneBody,
    UnknownNames::Lets,
    UnknownNames::Repeated,
    UnknownNames::Restored,
    UnknownNames::Valid,
];

impl UnknownNames {
    fn items(self, n: usize) -> Vec<String> {
        let mut items = vec!["state note(60)\n".to_string()];
        let line = |i: usize| match self {
            UnknownNames::OneBody | UnknownNames::Lets => format!("    Note/(q{i})\n"),
            UnknownNames::Repeated => format!("    Note/(q{})\n", i % 10),
            UnknownNames::Restored => format!("    Note when q{i} or 1 or 2\n"),
            UnknownNames::Valid => "    Note/(note)\n".to_string(),
        };
        if self == UnknownNames::Lets {
            for i in 0..n {
                items.push(format!("let v{i} = q{i} + 1\n"));
            }
            let reads = (0..n)
                .map(|i| format!("    Note/(v{i})\n"))
                .collect::<String>();
            items.push(format!("Clip c:\n{reads}"));
        } else {
            items.push(format!("Clip c:\n{}", (0..n).map(line).collect::<String>()));
        }
        items
    }

    fn expected(self, n: usize) -> (usize, usize) {
        match self {
            UnknownNames::OneBody | UnknownNames::Lets | UnknownNames::Repeated => (n, n),
            UnknownNames::Restored => (n, 2 * n),
            UnknownNames::Valid => (0, 0),
        }
    }
}

fn unknown_names_failures(sizes: [usize; 3]) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut rows = 0;
    for names in UNKNOWN_NAMES {
        for reversed in [false, true] {
            rows += 1;
            let name = format!("298 {names:?} reversed {reversed}");
            let texts = sizes.map(|n| {
                let mut items = names.items(n);
                if reversed {
                    items.reverse();
                }
                items.concat()
            });
            let runs = texts.each_ref().map(|text| measured(&compiler, text));
            for (run, n) in runs.iter().zip(sizes) {
                let unknown = run
                    .errors
                    .iter()
                    .filter(|error| error.starts_with("UnknownName"))
                    .count();
                if (unknown, run.errors.len()) != names.expected(n) {
                    failures.push(format!(
                        "{name} {n}: {} errors, {unknown} unknown-name, the first {:?}",
                        run.errors.len(),
                        run.errors.first()
                    ));
                }
            }
            for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
                let points = [0, 1, 2].map(|run| {
                    (
                        u64::try_from(texts[run].len()).unwrap_or(u64::MAX),
                        runs[run].counts.get(index).map_or(0, |(_, count)| *count),
                    )
                });
                if !linear(points) {
                    failures.push(format!(
                        "{name}: {counter} {:?}",
                        points.map(|(_, count)| count)
                    ));
                }
            }
        }
    }
    assert_eq!(rows, 10);
    failures
}

#[test]
fn distinct_unknown_names_cost_work_linear_in_the_names_and_the_sources_298() {
    let failures = unknown_names_failures([20, 40, 80]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "stress"]
fn stress_distinct_unknown_names_cost_work_linear_in_the_names_and_the_sources_298() {
    let failures = unknown_names_failures([8_000, 16_000, 32_000]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_cost_counters_hold_the_signature_walk_and_it_is_linear_in_a_recursive_calls_references_320()
{
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for body in [ReferenceBody::Length, ReferenceBody::ChosenGain] {
        for call_first in [true, false] {
            let sizes = [25, 50, 100];
            let texts = sizes.map(|size| reference_array_program(body, size, call_first));
            let runs = texts.each_ref().map(|text| measured(&compiler, text));
            let counted = |run: &Measured| {
                run.counts
                    .iter()
                    .find(|(counter, _)| counter == "SignatureRunSig")
                    .map(|(_, count)| *count)
            };
            let points = [0, 1, 2].map(|run| {
                (
                    u64::try_from(texts[run].len()).unwrap_or(u64::MAX),
                    counted(&runs[run]).unwrap_or(0),
                )
            });
            if points.iter().any(|(_, count)| *count == 0) || !linear(points) {
                failures.push(format!("{body:?} call first {call_first}: {points:?}"));
            }
            for (run, size) in runs.iter().zip(sizes) {
                if let Some(first) = run.errors.first() {
                    failures.push(format!("{body:?} {size}: {first}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn empty_chain_program(lets: usize, reads: usize) -> String {
    let mut text = String::from("state n(0)\nlet a0 = [1, 2][2..]\n");
    for level in 1..=lets {
        text.push_str(&format!("let a{level} = a{}[0..]\n", level - 1));
    }
    text.push_str("Clip c:\n");
    for _ in 0..reads {
        text.push_str(&format!(
            "    Note/(60 + a{lets}.size)\n    Note | velocity=(a{lets}[n] + 0.5)\n"
        ));
    }
    text
}

fn empty_chain_walks(sizes: [(usize, usize); 3]) -> (Vec<String>, [u64; 3]) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let runs = sizes.map(|(lets, reads)| measured(&compiler, &empty_chain_program(lets, reads)));
    let mut failures = Vec::new();
    let steps = runs.each_ref().map(|run| {
        run.counts
            .iter()
            .find(|(name, _)| name == "PathsSlicedGroup")
            .map_or(0, |(_, count)| *count)
    });
    for (run, size) in runs.iter().zip(sizes) {
        if let Some(first) = run.errors.first() {
            failures.push(format!("388 {size:?}: {first}"));
        }
    }
    (failures, steps)
}

#[test]
fn an_empty_arrays_let_chain_is_walked_once_per_let_whatever_reads_it_388() {
    let mut failures = Vec::new();
    for (sizes, measure) in [
        ([(25, 4), (50, 4), (100, 4)], 0),
        ([(4, 25), (4, 50), (4, 100)], 1),
        ([(25, 25), (50, 50), (100, 100)], 0),
    ] {
        let (found, steps) = empty_chain_walks(sizes);
        failures.extend(found);
        let points = [0, 1, 2].map(|run| {
            let (lets, reads) = sizes[run];
            (
                u64::try_from(if measure == 0 { lets } else { reads }).unwrap_or(u64::MAX),
                steps[run],
            )
        });
        if !linear(points) {
            failures.push(format!("388 {sizes:?}: PathsSlicedGroup {steps:?}"));
        }
        if steps[0] == 0 {
            failures.push(format!("388 {sizes:?}: the chain is never walked"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const DISTINCT_CALLS: &str = "fn f(a, b, c): if a < 1 then 0 else f(a - 1, b + 1, c) + f(a - 1, b, c + 1) + f(a - 1, b, c)\nClip k:\n    Note/(f(200, 0, 0))\n";

fn distinct_call_failures(calls: [u64; 3]) -> Vec<String> {
    let mut runs = Vec::new();
    for limit in calls {
        let limits = LimitOverrides {
            recursive_calls: Some(limit),
            recursive_calls_warning: Some(limit),
            ..LimitOverrides::default()
        };
        let compiler =
            Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
        runs.push(measured(&compiler, DISTINCT_CALLS));
    }
    let mut failures = Vec::new();
    for (run, limit) in runs.iter().zip(calls) {
        let limited = run.errors.len() == 1
            && run
                .errors
                .first()
                .is_some_and(|error| error.starts_with("RecursionLimit"));
        if !limited {
            failures.push(format!("300 at {limit} calls: {:?}", run.errors));
        }
    }
    for (index, (counter, _)) in runs[0].counts.iter().enumerate() {
        let points = [0, 1, 2].map(|run| {
            (
                calls[run],
                runs[run].counts.get(index).map_or(0, |(_, count)| *count),
            )
        });
        if !linear(points) {
            failures.push(format!(
                "300 {counter} {:?} at {calls:?} calls",
                points.map(|(_, count)| count)
            ));
        }
    }
    failures
}

#[test]
fn a_compile_time_recursive_calls_work_is_the_same_for_every_distinct_call_300() {
    let failures = distinct_call_failures([2_000, 4_000, 8_000]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore]
fn stress_a_compile_time_recursive_calls_work_is_the_same_for_every_distinct_call_up_to_the_limit_300()
 {
    let failures = distinct_call_failures([250_000, 500_000, 1_000_000]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
