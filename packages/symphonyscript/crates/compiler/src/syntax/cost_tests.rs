use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::vocabulary;

use super::lexer::Lexer;
use super::parser::tests::{NESTS, Nest, nest_program};
use super::syntax_kind::SyntaxKind;
use crate::counting_allocator::CountingAllocator;
use crate::semantics::counted_walk::CountedWalk;
use crate::source::Source;

const WALKS: [CountedWalk; 4] = [
    CountedWalk::LexByte,
    CountedWalk::LexToken,
    CountedWalk::ParseKindRead,
    CountedWalk::ParseNode,
];

const TOKEN_BYTES: u64 = 6;
const SIGNIFICANT_BYTES: u64 = 4;
const NODE_BYTES: u64 = 14;
const LAZY_NODE_BYTES: u64 = 8;
const LAZY_TOKEN_BYTES: u64 = 4;

#[derive(Debug)]
struct Counted {
    bytes: u64,
    tokens: u64,
    layout: u64,
    nodes: u64,
    steps: [u64; 4],
}

fn source(text: &str) -> Source {
    Source {
        path: "cost.sym".into(),
        text: text.into(),
    }
}

fn wide(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

fn counted(text: &str, vocabulary: &Vocabulary) -> Counted {
    let before = WALKS.map(CountedWalk::steps);
    let parsed = super::parse(&source(text), vocabulary);
    let after = WALKS.map(CountedWalk::steps);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let kinds = &parsed.tokens().kinds;
    let layout = kinds
        .iter()
        .filter(|kind| {
            matches!(
                kind,
                SyntaxKind::Indent | SyntaxKind::Dedent | SyntaxKind::Newline
            )
        })
        .count();
    Counted {
        bytes: wide(text.len()),
        tokens: wide(kinds.len()),
        layout: wide(layout),
        nodes: wide(parsed.tree().len()),
        steps: [0, 1, 2, 3].map(|walk| after[walk] - before[walk]),
    }
}

fn units(walk: usize, counted: &Counted) -> u64 {
    match walk {
        0 => counted.bytes,
        1 => counted.bytes + counted.layout,
        _ => counted.tokens,
    }
}

const BOUNDS: [u64; 4] = [4, 1, 5, 1];

fn linear_failures(nest: Nest, sizes: [usize; 3]) -> (Vec<String>, Vec<String>) {
    let vocabulary = vocabulary();
    let runs = sizes.map(|n| counted(&nest_program(nest, n), &vocabulary));
    let mut failures = Vec::new();
    let mut constants = Vec::new();
    for (walk, name) in WALKS.iter().enumerate() {
        let counts = runs.each_ref().map(|run| run.steps[walk]);
        let per_unit: Vec<String> = runs
            .iter()
            .map(|run| {
                format!(
                    "{:.3}",
                    run.steps[walk] as f64 / units(walk, run).max(1) as f64
                )
            })
            .collect();
        constants.push(format!(
            "{nest:?} {name:?} {counts:?} per unit {per_unit:?}"
        ));
        if counts.contains(&0) {
            failures.push(format!("{nest:?} {name:?}: no step at {counts:?}"));
        }
        for pair in counts.windows(2) {
            if pair[1] > 2 * pair[0] + 64 {
                failures.push(format!("{nest:?} {name:?}: past linear {counts:?}"));
            }
        }
        for run in &runs {
            if run.steps[walk] > BOUNDS[walk] * units(walk, run) {
                failures.push(format!(
                    "{nest:?} {name:?}: {} steps past {} a unit of {}",
                    run.steps[walk],
                    BOUNDS[walk],
                    units(walk, run)
                ));
            }
        }
    }
    for run in &runs {
        if run.steps[1] != run.tokens || run.nodes != run.steps[3] {
            failures.push(format!("{nest:?}: the steps miss the columns {run:?}"));
        }
    }
    (failures, constants)
}

fn assert_linear(sizes: [usize; 3]) {
    let mut failures = Vec::new();
    for nest in NESTS {
        let (found, constants) = linear_failures(nest, sizes);
        for line in constants {
            println!("{line}");
        }
        failures.extend(found);
    }
    crate::pinned_row::assert_no_failure(&failures);
}

#[test]
fn lexing_and_parsing_the_figure_programs_cost_work_linear_in_their_bytes_and_tokens() {
    assert_linear([500, 1_000, 2_000]);
}

#[test]
#[ignore = "stress"]
fn stress_lexing_and_parsing_the_figure_programs_cost_work_linear_in_their_bytes_and_tokens() {
    assert_linear([25_000, 50_000, 100_000]);
}

#[derive(Debug)]
struct Memory {
    tokens: u64,
    significant: u64,
    nodes: u64,
    text: u64,
    lexed: u64,
    lex_peak: u64,
    parsed: u64,
    parse_peak: u64,
    lazy: u64,
}

fn memory(text: &str, vocabulary: &Vocabulary) -> Memory {
    let source = source(text);
    let started = CountingAllocator::begin();
    let mut lexer = Lexer::new(&source, vocabulary);
    while lexer.advance() {}
    let (columns, _) = lexer.finish();
    let lexed = CountingAllocator::held_since(started);
    let (_, lex_peak) = CountingAllocator::since(started);
    let (tokens, significant) = (wide(columns.len()), wide(columns.sig.len()));
    drop(columns);
    let started = CountingAllocator::begin();
    let parsed = super::parse(&source, vocabulary);
    let parsed_bytes = CountingAllocator::held_since(started);
    let (_, parse_peak) = CountingAllocator::since(started);
    let started = CountingAllocator::begin();
    let first = parsed.root().first_token().and_then(|token| token.parent());
    let second = first.and_then(|node| node.parent());
    let lazy = CountingAllocator::held_since(started);
    assert!(second.is_some());
    Memory {
        tokens,
        significant,
        nodes: wide(parsed.tree().len()),
        text: wide(text.len()),
        lexed,
        lex_peak,
        parsed: parsed_bytes,
        parse_peak,
        lazy,
    }
}

fn ratio(bytes: u64, units: u64) -> f64 {
    bytes as f64 / units.max(1) as f64
}

#[test]
fn lexing_and_parsing_the_figure_programs_hold_their_columns_within_twice_the_layouts_bytes() {
    let vocabulary = vocabulary();
    let mut failures = Vec::new();
    for nest in NESTS {
        let found = memory(&nest_program(nest, 2_000), &vocabulary);
        let token_budget = TOKEN_BYTES * found.tokens + SIGNIFICANT_BYTES * found.significant;
        let node_budget = NODE_BYTES * found.nodes;
        let lazy_budget = LAZY_NODE_BYTES * found.nodes + LAZY_TOKEN_BYTES * found.tokens;
        let tree = found.parsed.saturating_sub(found.lexed + found.text);
        println!(
            "{nest:?} {found:?}: lexing holds {:.2} B a token, parsing {:.2} B a node beside the tokens and the text, the lazy columns {:.2} B a node and token; at peak lexing {:.2} B a token, parsing {:.2} B a node",
            ratio(found.lexed, found.tokens),
            ratio(tree, found.nodes),
            ratio(found.lazy, found.nodes + found.tokens),
            ratio(found.lex_peak, found.tokens),
            ratio(found.parse_peak.saturating_sub(found.text), found.nodes),
        );
        if found.lexed > 2 * token_budget {
            failures.push(format!(
                "{nest:?}: lexing {found:?} past {}",
                2 * token_budget
            ));
        }
        if found.parsed > 2 * (token_budget + node_budget) + found.text {
            failures.push(format!(
                "{nest:?}: parsing {found:?} past {}",
                2 * (token_budget + node_budget) + found.text
            ));
        }
        if found.lazy > 2 * lazy_budget {
            failures.push(format!(
                "{nest:?}: the lazy columns {found:?} past {}",
                2 * lazy_budget
            ));
        }
    }
    crate::pinned_row::assert_no_failure(&failures);
}
