use std::process::Command;

use serde_json::{Value, json};
use symphonyscript_core::LimitOverrides;
use symphonyscript_evolve::arguments;

const TWO_HUNDRED_CALLS: &str =
    "state i(1)\nfn f(x, k): x + k\nClip c:\n    Note/(f(i, 0) + f(i, 1))\n";

fn given(text: &str, notes: usize) -> String {
    format!("{text}{}", "    Note/(f(i, 0) + f(i, 1))\n".repeat(notes))
}

fn codes(name: &str, text: &str, flags: &[&str]) -> Value {
    let given = format!("{}/{name}.in.jsonl", env!("CARGO_TARGET_TMPDIR"));
    let taken = format!("{}/{name}.out.jsonl", env!("CARGO_TARGET_TMPDIR"));
    let line = json!({"id": name, "sources": [{"path": "main.sym", "text": text}]});
    if let Err(error) = std::fs::write(&given, format!("{line}\n")) {
        panic!("{given}: {error}");
    }
    let output = match Command::new(env!("CARGO_BIN_EXE_evolve-time"))
        .args(["--budget", "600"])
        .args(flags)
        .args([given.as_str(), taken.as_str()])
        .output()
    {
        Ok(output) => output,
        Err(error) => panic!("evolve-time does not start: {error}"),
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::fs::read_to_string(&taken).unwrap_or_else(|error| panic!("{taken}: {error}"));
    let time: Value =
        serde_json::from_str(text.trim()).unwrap_or_else(|error| panic!("{text}: {error}"));
    time["codes"].clone()
}

fn parsed(raw: &[&str]) -> Result<LimitOverrides, String> {
    arguments(raw.iter().map(|word| word.to_string())).map(|parsed| parsed.limits)
}

#[test]
fn a_limit_flag_sets_that_limit_for_every_program_the_run_compiles() {
    let program = given(TWO_HUNDRED_CALLS, 99);
    assert_eq!(
        codes("limits-default", &program, &[]),
        json!(["no-entry-instance"])
    );
    assert_eq!(
        codes(
            "limits-lowered",
            &program,
            &[
                "--limit",
                "inlined_calls=150",
                "--limit",
                "inlined_calls_warning=100"
            ]
        ),
        json!(["inline-limit", "limit-warning"])
    );
    assert_eq!(
        codes(
            "limits-warned",
            &program,
            &["--limit", "inlined_calls_warning=150"]
        ),
        json!(["limit-warning", "no-entry-instance"])
    );
}

#[test]
fn a_limit_flag_names_a_limits_field_and_takes_a_whole_number() {
    assert_eq!(
        parsed(&[
            "--limit",
            "inlined_calls=1000000",
            "--limit",
            "inlined_calls_warning=500000"
        ]),
        Ok(LimitOverrides {
            inlined_calls: Some(1_000_000),
            inlined_calls_warning: Some(500_000),
            ..LimitOverrides::default()
        })
    );
    assert_eq!(parsed(&[]), Ok(LimitOverrides::default()));
    assert_eq!(
        parsed(&["--limit", "inline=5"]),
        Err("--limit names no limit `inline`".to_string())
    );
    assert_eq!(
        parsed(&["--limit", "inlined_calls"]),
        Err("--limit takes <name>=<value>, not `inlined_calls`".to_string())
    );
    assert_eq!(
        parsed(&["--limit", "inlined_calls=-1"]),
        Err("--limit takes <name>=<value>, not `inlined_calls=-1`".to_string())
    );
    assert_eq!(
        parsed(&["--limit"]),
        Err("--limit takes <name>=<value>".to_string())
    );
}
