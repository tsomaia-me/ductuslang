use symphonyscript_fixtures::vocabulary;

use super::super::Parser;
use crate::semantics::counted_walk::CountedWalk;
use crate::source::Source;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Nest {
    Parentheses,
    ArrayAndIndex,
    Call,
    Not,
    Negation,
    Interpolation,
    Inlined,
}

pub(crate) const NESTS: [Nest; 7] = [
    Nest::Parentheses,
    Nest::ArrayAndIndex,
    Nest::Call,
    Nest::Not,
    Nest::Negation,
    Nest::Interpolation,
    Nest::Inlined,
];

fn nested(opening: &str, middle: &str, closing: &str, depth: usize) -> String {
    format!("{}{middle}{}", opening.repeat(depth), closing.repeat(depth))
}

pub(super) fn nest_programs(n: usize) -> Vec<String> {
    NESTS.iter().map(|nest| nest_program(*nest, n)).collect()
}

pub(crate) fn nest_program(nest: Nest, n: usize) -> String {
    let depth = |of: Nest| {
        if matches!(
            (nest, of),
            (Nest::Parentheses, Nest::Parentheses)
                | (Nest::ArrayAndIndex, Nest::ArrayAndIndex)
                | (Nest::Call, Nest::Call)
                | (Nest::Not, Nest::Not)
                | (Nest::Negation, Nest::Negation)
                | (Nest::Interpolation, Nest::Interpolation)
        ) {
            n
        } else {
            1
        }
    };
    if let Nest::Inlined = nest {
        let calls: Vec<String> = (0..100).map(|index| format!("f(i, {index})")).collect();
        let note = format!("    Note/({})\n", calls.join(" + "));
        return format!(
            "state i(1)\nfn f(x, k): x + k\nClip c:\n{}",
            note.repeat(n / 100)
        );
    }
    let array = depth(Nest::ArrayAndIndex);
    format!(
        "state x(true)\nfn f(v): v\nlet a = {}\nlet b = {}\nlet c = {}\nlet d = {}x\nlet e = {}\nlet g = {}1\nClip top:\n    Note/(60) when d\n    Note/(f(a) + c + g + b{})\n",
        nested("(", "1", ")", depth(Nest::Parentheses)),
        nested("[", "1", "]", array),
        nested("f(", "1", ")", depth(Nest::Call)),
        "not ".repeat(depth(Nest::Not)),
        nested("\"{", "1", "}\"", depth(Nest::Interpolation)),
        "-".repeat(depth(Nest::Negation)),
        "[0]".repeat(array)
    )
}

fn kind_reads(text: &str) -> (u64, u64) {
    let source = Source {
        path: "main.sym".into(),
        text: text.into(),
    };
    let vocabulary = vocabulary();
    let before = CountedWalk::ParseKindRead.steps();
    let (_, diagnostics, cost) = Parser::new(&source, &vocabulary).run_counting();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    (
        u64::try_from(cost.tokens).unwrap_or(u64::MAX),
        CountedWalk::ParseKindRead.steps() - before,
    )
}

#[test]
fn the_parser_reads_each_tokens_kind_a_constant_number_of_times_429() {
    let mut failures = Vec::new();
    for nest in NESTS {
        let counted = [2_000, 4_000, 8_000].map(|n| kind_reads(&nest_program(nest, n)));
        for pair in counted.windows(2) {
            if pair[1].1 > 2 * pair[0].1 + 64 {
                failures.push(crate::pinned_row::failed_counts(
                    &nest_program(nest, 2_000),
                    &[],
                    format!("{nest:?}: the kind reads grow past the tokens {counted:?}"),
                ));
            }
        }
        if counted
            .iter()
            .any(|(tokens, reads)| *reads == 0 || *reads > 5 * tokens)
        {
            failures.push(crate::pinned_row::failed_counts(
                &nest_program(nest, 2_000),
                &[],
                format!("{nest:?}: past 5 kind reads a token {counted:?}"),
            ));
        }
    }
    crate::pinned_row::assert_no_failure(&failures);
}
