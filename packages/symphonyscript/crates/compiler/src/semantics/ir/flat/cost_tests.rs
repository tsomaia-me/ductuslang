use symphonyscript_fixtures::{prelude_vocabulary, vocabulary};

use super::{FlatIr, build_flat_file};
use crate::compiler::Compiler;
use crate::counting_allocator::CountingAllocator;
use crate::semantics::counted_walk::CountedWalk;
use crate::source::Source;
use crate::syntax::{SyntaxKind, parse};

#[derive(Clone, Copy, Debug)]
enum Nest {
    Parentheses,
    ArrayAndIndex,
    Call,
    Not,
    Inlined,
}

const NESTS: [Nest; 5] = [
    Nest::Parentheses,
    Nest::ArrayAndIndex,
    Nest::Call,
    Nest::Not,
    Nest::Inlined,
];

fn nest_program(nest: Nest, n: usize) -> String {
    let nested = |opening: &str, middle: &str, closing: &str| {
        format!("{}{middle}{}", opening.repeat(n), closing.repeat(n))
    };
    let (a, b, c, d, index) = match nest {
        Nest::Parentheses => (
            nested("(", "1", ")"),
            "[1]".into(),
            "f(1)".into(),
            "not ".into(),
            "[0]".into(),
        ),
        Nest::ArrayAndIndex => (
            "(1)".into(),
            nested("[", "1", "]"),
            "f(1)".into(),
            "not ".into(),
            "[0]".repeat(n),
        ),
        Nest::Call => (
            "(1)".into(),
            "[1]".into(),
            nested("f(", "1", ")"),
            "not ".into(),
            "[0]".into(),
        ),
        Nest::Not => (
            "(1)".into(),
            "[1]".into(),
            "f(1)".into(),
            "not ".repeat(n),
            "[0]".into(),
        ),
        Nest::Inlined => {
            let calls: Vec<String> = (0..100).map(|index| format!("f(i, {index})")).collect();
            let note = format!("    Note/({})\n", calls.join(" + "));
            return format!(
                "state i(1)\nfn f(x, k): x + k\nClip c:\n{}",
                note.repeat((n / 100).max(1))
            );
        }
    };
    format!(
        "state x(true)\nfn f(v): v\nlet a = {a}\nlet b = {b}\nlet c = {c}\nlet d = {d}x\nClip top:\n    Note/(60) when d\n    Note/(f(a) + c + b{index})\n"
    )
}

#[derive(Clone, Copy, Debug)]
struct Measured {
    syntax_nodes: u64,
    identifiers: u64,
    ir_nodes: u64,
    steps: u64,
    emitted: u64,
    probes: u64,
    held: u64,
    peak: u64,
}

fn count(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

fn measured(text: &str) -> Measured {
    let vocabulary = vocabulary();
    let source = Source {
        path: "main.sym".into(),
        text: text.into(),
    };
    let parsed = parse(&source, &vocabulary);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let syntax_nodes = count(parsed.tree().len());
    let identifiers = count(
        parsed
            .tokens()
            .kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::Ident)
            .count(),
    );
    let walks = [
        CountedWalk::IrBuilderStep,
        CountedWalk::IrNode,
        CountedWalk::HashProbe,
    ];
    let mut ir = FlatIr::default();
    let before = walks.map(CountedWalk::steps);
    let started = CountingAllocator::begin();
    assert_eq!(
        build_flat_file(&mut ir, "main.sym", &parsed, &vocabulary),
        Ok(())
    );
    let held = CountingAllocator::held_since(started);
    let (_, peak) = CountingAllocator::since(started);
    let [steps, emitted, probes] = [0, 1, 2].map(|at| walks[at].steps() - before[at]);
    Measured {
        syntax_nodes,
        identifiers,
        ir_nodes: count(ir.op.len()),
        steps,
        emitted,
        probes,
        held,
        peak,
    }
}

fn per(part: u64, whole: u64) -> f64 {
    part as f64 / whole.max(1) as f64
}

fn failures_at(sizes: [usize; 3]) -> Vec<String> {
    let mut failures = Vec::new();
    for nest in NESTS {
        let runs = sizes.map(|n| measured(&nest_program(nest, n)));
        for run in &runs {
            println!(
                "{nest:?} at {} syntax nodes: IrBuilderStep {:.3} a node, IrNode {:.3} a node, HashProbe {:.3} an identifier, {:.1} B held an IR node, {:.1} B peak a syntax node",
                run.syntax_nodes,
                per(run.steps, run.syntax_nodes),
                per(run.emitted, run.syntax_nodes),
                per(run.probes, run.identifiers),
                per(run.held, run.ir_nodes),
                per(run.peak, run.syntax_nodes),
            );
            if run.steps != run.syntax_nodes {
                failures.push(format!(
                    "{nest:?}: {} steps over {} syntax nodes",
                    run.steps, run.syntax_nodes
                ));
            }
            if run.emitted > run.syntax_nodes {
                failures.push(format!(
                    "{nest:?}: {} IR nodes over {} syntax nodes",
                    run.emitted, run.syntax_nodes
                ));
            }
            if run.probes > run.identifiers + 64 {
                failures.push(format!(
                    "{nest:?}: {} hash probes over {} identifiers",
                    run.probes, run.identifiers
                ));
            }
            if run.held > 50 * run.ir_nodes + 65_536 {
                failures.push(format!(
                    "{nest:?}: {} B held over {} IR nodes",
                    run.held, run.ir_nodes
                ));
            }
        }
        for pair in runs.windows(2) {
            if let [small, large] = pair
                && large.emitted > 2 * small.emitted + 64
            {
                failures.push(format!(
                    "{nest:?}: IR nodes grow past the doubling {small:?} {large:?}"
                ));
            }
        }
    }
    failures
}

#[test]
fn the_ir_build_takes_one_step_a_syntax_node_and_one_probe_an_identifier_430() {
    let failures = failures_at([400, 800, 1_600]);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
#[ignore = "stress"]
fn stress_the_ir_build_takes_one_step_a_syntax_node_and_one_probe_an_identifier_430() {
    let failures = failures_at([25_000, 50_000, 100_000]);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn two_compiles_on_one_compiler_build_the_preludes_ir_once() {
    let prelude = "fn double(x): x * 2\nlet base = double(30)\n";
    let program = "Clip c:\n    Note/(base + 1)\n";
    let nodes = |text: &str| {
        let source = Source {
            path: "count.sym".into(),
            text: text.into(),
        };
        count(parse(&source, &vocabulary()).tree().len())
    };
    let before = CountedWalk::IrBuilderStep.steps();
    let compiler = Compiler::new(&prelude_vocabulary(prelude)).expect("the prelude compiles");
    let after_new = CountedWalk::IrBuilderStep.steps();
    for _ in 0..2 {
        compiler.check(&[Source {
            path: "main.sym".into(),
            text: program.into(),
        }]);
    }
    let after_checks = CountedWalk::IrBuilderStep.steps();
    assert_eq!(after_new - before, nodes(prelude));
    assert_eq!(after_checks - after_new, 2 * nodes(program));
}
