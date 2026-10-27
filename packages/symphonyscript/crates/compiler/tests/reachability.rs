mod support;

use symphonyscript::Code;

#[test]
fn every_unreachable_top_level_instance_draws_its_warning() {
    let program = |clips: usize| {
        let mut text = String::from("Session s\n");
        for clip in 0..clips {
            text.push_str(&format!("Clip c{clip}\n"));
        }
        text
    };
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
