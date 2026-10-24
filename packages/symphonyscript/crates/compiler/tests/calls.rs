mod support;

use symphonyscript::{Code, Diagnostic};

fn spans(text: &str, diagnostics: &[Diagnostic]) -> Vec<(Code, String, Vec<String>)> {
    let piece = |range: &std::ops::Range<u32>| {
        text.get(range.start as usize..range.end as usize)
            .unwrap_or_default()
            .to_string()
    };
    diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                piece(&diagnostic.span.range),
                diagnostic
                    .secondary
                    .iter()
                    .map(|span| piece(&span.range))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn an_error_in_an_inlined_body_names_its_calls_innermost_first() {
    let fit = |call: &str| {
        (
            Code::DoesNotFit,
            "n * 2".to_string(),
            vec![call.to_string()],
        )
    };
    let loud = "fn loud(n): n * 2\nClip c:\n";
    let rows = [
        (
            format!(
                "{loud}    Note/(loud(2000000000))\n    Note/60\n    Note/(loud(1500000000))\n"
            ),
            vec![fit("loud(2000000000)"), fit("loud(1500000000)")],
        ),
        (
            format!("{loud}    Note/(loud(2000000000))\n    Note/(loud(2000000000))\n"),
            vec![fit("loud(2000000000)"), fit("loud(2000000000)")],
        ),
        (
            format!("{loud}    Note/(loud(2000000000))\n    Note/(loud(5))\n"),
            vec![fit("loud(2000000000)")],
        ),
    ];
    for (text, expected) in rows {
        let found = spans(&text, &support::checked(&text));
        assert_eq!(found, expected, "{text}");
    }
    let nested = "fn loud(n): n * 2\nfn twice(m): loud(m)\nClip c:\n    Note/(twice(2000000000))\n";
    assert_eq!(
        spans(nested, &support::checked(nested)),
        vec![(
            Code::DoesNotFit,
            "n * 2".to_string(),
            vec!["loud(m)".to_string(), "twice(2000000000)".to_string()]
        )]
    );
}

fn doubled_patterns(levels: usize, uses: usize) -> String {
    let mut text = String::from("pattern P0():\n    Note/300000000000\n");
    for level in 1..levels {
        let below = level - 1;
        text.push_str(&format!(
            "pattern P{level}():\n    P{below}\n    P{below}\n"
        ));
    }
    text.push_str("Clip c:\n");
    for _ in 0..uses {
        text.push_str(&format!("    P{}\n", levels - 1));
    }
    text
}

#[test]
fn a_fault_draws_one_diagnostic_per_outermost_call_or_use() {
    let fit = |chain: &[&str]| {
        (
            Code::DoesNotFit,
            "300000000000".to_string(),
            chain
                .iter()
                .map(|call| call.to_string())
                .collect::<Vec<_>>(),
        )
    };
    let once = doubled_patterns(12, 1);
    let uses: Vec<String> = (0..12).map(|level| format!("P{level}")).collect();
    let chain: Vec<&str> = uses.iter().map(String::as_str).collect();
    assert_eq!(spans(&once, &support::checked(&once)), vec![fit(&chain)]);
    let twice = doubled_patterns(12, 2);
    let found = spans(&twice, &support::checked(&twice));
    assert_eq!(found, vec![fit(&chain), fit(&chain)]);
    let positions: Vec<Option<u32>> = support::checked(&twice)
        .iter()
        .map(|diagnostic| diagnostic.secondary.last().map(|span| span.range.start))
        .collect();
    assert_ne!(positions.first(), positions.last());
    let out_of_range = |chain: &[&str]| {
        (
            Code::IndexOutOfRange,
            "n".to_string(),
            chain
                .iter()
                .map(|call| call.to_string())
                .collect::<Vec<_>>(),
        )
    };
    let shared_inner =
        "fn pick(n): [1, 2][n]\nfn both(x): pick(x) + pick(x)\nClip c:\n    Note/(both(5))\n";
    let diagnostics = support::checked(shared_inner);
    assert_eq!(
        spans(shared_inner, &diagnostics),
        vec![out_of_range(&["pick(x)", "both(5)"])]
    );
    let first_pick = shared_inner
        .find("pick(x)")
        .and_then(|at| u32::try_from(at).ok());
    assert_eq!(
        diagnostics
            .first()
            .and_then(|diagnostic| diagnostic.secondary.first())
            .map(|span| span.range.start),
        first_pick
    );
    let siblings = "fn pick(n): [1, 2][n]\nClip c:\n    Note/(pick(5) + pick(5))\n";
    let found = spans(siblings, &support::checked(siblings));
    assert_eq!(
        found,
        vec![out_of_range(&["pick(5)"]), out_of_range(&["pick(5)"])]
    );
    let starts: Vec<Option<u32>> = support::checked(siblings)
        .iter()
        .map(|diagnostic| diagnostic.secondary.first().map(|span| span.range.start))
        .collect();
    assert_ne!(starts.first(), starts.last());
}
