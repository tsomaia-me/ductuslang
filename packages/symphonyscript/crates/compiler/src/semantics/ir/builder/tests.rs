use symphonyscript_fixtures::vocabulary;

use super::build_file::build_file;
use super::faults::Faults;
use super::hir_builder::HirBuilder;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::{BlockId, Hir};
use crate::source::Source;
use crate::syntax::ast::Item;
use crate::syntax::parse;

#[test]
fn a_write_into_a_block_the_builder_never_allocated_is_an_internal_error() {
    let vocabulary = vocabulary();
    let source = Source {
        path: "test.sym".into(),
        text: "Clip c:\n    Note\n".into(),
    };
    let parsed = parse(&source, &vocabulary);
    let Some(Item::Placement(placement)) = parsed.file().and_then(|file| file.items().next())
    else {
        panic!("the source holds one placement");
    };
    let Some(block) = placement.body().and_then(|body| body.block()) else {
        panic!("the placement has a block");
    };
    let mut hir = Hir::default();
    let mut builder = HirBuilder {
        hir: &mut hir,
        file: 0,
        faults: Faults::new(&parsed.diagnostics, &parsed.detached, &parsed.root()),
        vocabulary: &vocabulary,
        dropped: false,
    };
    assert_eq!(
        builder.fill(vec![(block.clone(), BlockId(0))]),
        Err(Internal::missed("block"))
    );
    let allocated = builder.empty_block();
    assert_eq!(builder.fill(vec![(block, allocated)]), Ok(()));
    assert_eq!(hir.blocks.len(), 1);
    assert_eq!(hir.blocks.first().map(|block| block.items.len()), Some(1));
}

#[derive(Clone, Copy, Debug)]
enum Nest {
    Parentheses,
    ArrayAndIndex,
    Call,
    Not,
    Inlined,
}

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
                note.repeat(n / 100)
            );
        }
    };
    format!(
        "state x(true)\nfn f(v): v\nlet a = {a}\nlet b = {b}\nlet c = {c}\nlet d = {d}x\nClip top:\n    Note/(60) when d\n    Note/(f(a) + c + b{index})\n"
    )
}

fn builder_counts(text: &str) -> [u64; 4] {
    let vocabulary = vocabulary();
    let source = Source {
        path: "main.sym".into(),
        text: text.into(),
    };
    let parsed = parse(&source, &vocabulary);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let nodes = u64::try_from(parsed.root().descendants().count()).unwrap_or(u64::MAX);
    let mut hir = Hir::default();
    let walks = [
        CountedWalk::HirBuilderCall,
        CountedWalk::FaultsUnclosed,
        CountedWalk::SyntaxViewElement,
    ];
    let before = walks.map(CountedWalk::steps);
    assert_eq!(
        build_file(&mut hir, "main.sym", &parsed, &vocabulary),
        Ok(())
    );
    let [calls, scans, elements] = [0, 1, 2].map(|index| walks[index].steps() - before[index]);
    [nodes, calls, scans, elements]
}

#[test]
fn the_hir_build_does_constant_work_per_syntax_node_430() {
    let bounds = [
        ("HirBuilderCall", 6),
        ("FaultsUnclosed", 1),
        ("SyntaxViewElement", 9),
    ];
    let mut failures = Vec::new();
    for nest in [
        Nest::Parentheses,
        Nest::ArrayAndIndex,
        Nest::Call,
        Nest::Not,
        Nest::Inlined,
    ] {
        let counted = [2_000, 4_000, 8_000].map(|n| builder_counts(&nest_program(nest, n)));
        for (index, (walk, most)) in bounds.iter().enumerate() {
            let points = counted.map(|counts| (counts[0], counts[index + 1]));
            for pair in points.windows(2) {
                if pair[1].1 > 2 * pair[0].1 + 64 {
                    failures.push(format!("{nest:?} {walk}: grows past the nodes {points:?}"));
                }
            }
            if points.iter().any(|(nodes, count)| count > &(most * nodes)) {
                failures.push(format!("{nest:?} {walk}: past {most} a node {points:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
