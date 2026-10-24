mod support;

use std::time::{Duration, Instant};

use symphonyscript::Code;

#[test]
fn reachability_warnings_cost_linear_time() {
    let program = |clips: usize| {
        let mut text = String::from("Session s\n");
        for clip in 0..clips {
            text.push_str(&format!("Clip c{clip}\n"));
        }
        text
    };
    let timed = |text: &str| {
        let start = Instant::now();
        support::checked_all(text);
        start.elapsed()
    };
    let (small_text, large_text) = (program(10_000), program(40_000));
    let mut small = Duration::MAX;
    let mut large = Duration::MAX;
    for _ in 0..5 {
        small = small.min(timed(&small_text));
        large = large.min(timed(&large_text));
    }
    assert!(
        large <= small * 8,
        "{small:?} for 10,000 clips, {large:?} for 40,000"
    );
    let text = program(3);
    let warnings: Vec<(Code, &str)> = support::checked_all(&text)
        .iter()
        .map(|diagnostic| {
            let range = diagnostic.span.range.clone();
            (
                diagnostic.code,
                text.get(range.start as usize..range.end as usize)
                    .unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        warnings,
        vec![
            (Code::UnreachableInstance, "c0"),
            (Code::UnreachableInstance, "c1"),
            (Code::UnreachableInstance, "c2"),
        ]
    );
}
