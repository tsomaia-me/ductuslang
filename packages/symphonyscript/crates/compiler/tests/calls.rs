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
    let shared =
        "fn pick(n): [1, 2][n]\nfn both(x): pick(x) + pick(x)\nClip c:\n    Note/(both(5))\n";
    let out_of_range = |call: &str| {
        (
            Code::IndexOutOfRange,
            "n".to_string(),
            vec![call.to_string(), "both(5)".to_string()],
        )
    };
    let mut found = spans(shared, &support::checked(shared));
    found.sort();
    assert_eq!(
        found,
        vec![out_of_range("pick(x)"), out_of_range("pick(x)")]
    );
    let positions: Vec<Vec<u32>> = support::checked(shared)
        .iter()
        .map(|diagnostic| {
            diagnostic
                .secondary
                .iter()
                .map(|span| span.range.start)
                .collect()
        })
        .collect();
    assert_eq!(positions.len(), 2);
    assert_ne!(positions.first(), positions.last());
}
