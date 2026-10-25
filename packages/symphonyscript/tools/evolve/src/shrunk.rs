use symphonyscript::Source;
use symphonyscript::syntax::{NodeView, SyntaxKind, parse};

use crate::checker::Checker;
use crate::items::top_level_items;
use crate::program::{Program, ProgramSource};
use crate::signal::Signal;

const EXPRESSIONS: &[SyntaxKind] = &[
    SyntaxKind::BinaryExpr,
    SyntaxKind::PrefixExpr,
    SyntaxKind::ParenExpr,
    SyntaxKind::IfExpr,
    SyntaxKind::CallExpr,
    SyntaxKind::IndexExpr,
    SyntaxKind::FieldExpr,
    SyntaxKind::Conversion,
    SyntaxKind::ArrayLit,
    SyntaxKind::RangeExpr,
    SyntaxKind::ToExpr,
];

const OPERANDS: &[SyntaxKind] = &[
    SyntaxKind::Literal,
    SyntaxKind::NameRef,
    SyntaxKind::StringLit,
    SyntaxKind::VariantRef,
];

pub fn shrunk(checker: &Checker, program: &Program, signal: Signal) -> Program {
    let holds = |candidate: &Program| checker.holds(candidate, signal);
    let mut current = program.clone();
    loop {
        let size = current.size();
        let items: Vec<(usize, String)> = current
            .sources
            .iter()
            .enumerate()
            .flat_map(|(index, source)| {
                top_level_items(source, checker.vocabulary())
                    .into_iter()
                    .map(move |item| (index, item))
            })
            .collect();
        current = rebuilt(
            &current,
            &reduced(items, |units| holds(&rebuilt(&current, units))),
        );
        let lines: Vec<(usize, String)> = current
            .sources
            .iter()
            .enumerate()
            .flat_map(|(index, source)| {
                source
                    .text
                    .split_inclusive('\n')
                    .map(move |line| (index, line.to_string()))
            })
            .collect();
        current = rebuilt(
            &current,
            &reduced(lines, |units| holds(&rebuilt(&current, units))),
        );
        current = shortened(checker, current, &holds);
        if current.size() >= size {
            return current;
        }
    }
}

fn rebuilt(program: &Program, units: &[(usize, String)]) -> Program {
    let sources = program
        .sources
        .iter()
        .enumerate()
        .map(|(index, source)| ProgramSource {
            path: source.path.clone(),
            text: units
                .iter()
                .filter(|(file, _)| *file == index)
                .map(|(_, text)| text.as_str())
                .collect(),
        })
        .collect();
    program.with_sources(sources)
}

fn reduced<T: Clone>(mut units: Vec<T>, holds: impl Fn(&[T]) -> bool) -> Vec<T> {
    let mut size = units.len().div_ceil(2);
    while size > 0 {
        let mut start = 0;
        while start < units.len() {
            let end = start + size;
            let candidate: Vec<T> = units
                .iter()
                .enumerate()
                .filter(|(index, _)| *index < start || *index >= end)
                .map(|(_, unit)| unit.clone())
                .collect();
            if holds(&candidate) {
                units = candidate;
            } else {
                start = end;
            }
        }
        size /= 2;
    }
    units
}

fn shortened(
    checker: &Checker,
    mut program: Program,
    holds: &impl Fn(&Program) -> bool,
) -> Program {
    'restart: loop {
        for (index, source) in program.sources.iter().enumerate() {
            for (range, replacement) in replacements(checker, source) {
                let (Some(before), Some(after)) =
                    (source.text.get(..range.start), source.text.get(range.end..))
                else {
                    continue;
                };
                let text = format!("{before}{replacement}{after}");
                let mut sources = program.sources.clone();
                if let Some(target) = sources.get_mut(index) {
                    target.text = text;
                }
                let candidate = program.with_sources(sources);
                if holds(&candidate) {
                    program = candidate;
                    continue 'restart;
                }
            }
        }
        return program;
    }
}

fn replacements(
    checker: &Checker,
    source: &ProgramSource,
) -> Vec<(std::ops::Range<usize>, String)> {
    let parsed = parse(
        &Source {
            path: source.path.clone(),
            text: source.text.clone(),
        },
        checker.vocabulary(),
    );
    let mut found = Vec::new();
    let mut pending: Vec<NodeView<'_>> = vec![parsed.root()];
    while let Some(node) = pending.pop() {
        let children: Vec<NodeView<'_>> = node.children().collect();
        if EXPRESSIONS.contains(&node.kind())
            && let (Ok(start), Ok(end)) = (
                usize::try_from(node.range().start),
                usize::try_from(node.range().end),
            )
        {
            for operand in operands(&children) {
                found.push((start..end, operand.text()));
            }
        }
        pending.extend(children.into_iter().rev());
    }
    found
}

fn operands<'p>(children: &[NodeView<'p>]) -> Vec<NodeView<'p>> {
    let mut operands = Vec::new();
    for child in children {
        if child.kind() == SyntaxKind::ArgList {
            operands.extend(child.children().filter(is_operand));
        } else if is_operand(child) {
            operands.push(child.clone());
        }
    }
    operands
}

fn is_operand(node: &NodeView<'_>) -> bool {
    EXPRESSIONS.contains(&node.kind()) || OPERANDS.contains(&node.kind())
}
