use std::process::Command;
use std::time::Duration;

use serde_json::{Value, json};
use symphonyscript_evolve::{Guarded, abandoned, guarded, running_compiles};

const CALLS_PER_NOTE: usize = 100;
const LIGHT: &str = "Clip c:\n    Note\n";

fn inlining(notes: usize) -> String {
    let calls: Vec<String> = (0..CALLS_PER_NOTE)
        .map(|index| format!("f(i, {index})"))
        .collect();
    let note = format!("    Note/({})\n", calls.join(" + "));
    format!(
        "input i = 1\nfn f(x, k): x + k\nClip c:\n{}",
        note.repeat(notes)
    )
}

fn written(name: &str, programs: &[(&str, &str)]) -> (String, String) {
    let given = format!("{}/{name}.in.jsonl", env!("CARGO_TARGET_TMPDIR"));
    let taken = format!("{}/{name}.out.jsonl", env!("CARGO_TARGET_TMPDIR"));
    let lines: String = programs
        .iter()
        .map(|(id, text)| {
            format!(
                "{}\n",
                json!({"id": id, "sources": [{"path": "main.sym", "text": text}]})
            )
        })
        .collect();
    if let Err(error) = std::fs::write(&given, lines) {
        panic!("{given}: {error}");
    }
    (given, taken)
}

fn run(binary: &str, budget: &str, given: &str, taken: &str) -> Vec<Value> {
    let output = match Command::new(binary)
        .args(["--budget", budget, given, taken])
        .output()
    {
        Ok(output) => output,
        Err(error) => panic!("{binary} does not start: {error}"),
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::fs::read_to_string(taken).unwrap_or_else(|error| panic!("{taken}: {error}"));
    text.lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|error| panic!("{line}: {error}")))
        .collect()
}

#[test]
fn the_guard_counts_a_compile_it_abandons_until_that_compile_ends() {
    assert!(!abandoned());
    let outcome = guarded(Duration::from_millis(10), || {
        std::thread::sleep(Duration::from_millis(300));
    });
    assert_eq!(outcome, Guarded::TimedOut);
    assert!(abandoned());
    assert_eq!(running_compiles(), 1);
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(running_compiles(), 0);
    let within = guarded(Duration::from_secs(5), || 7);
    assert!(matches!(within, Guarded::Finished { value: 7, .. }));
    assert_eq!(running_compiles(), 0);
}

#[test]
fn a_compile_past_the_budget_leaves_no_running_compile_beside_the_next_program() {
    let heavy = inlining(1000);
    let (given, taken) = written(
        "time-past",
        &[("heavy", &heavy), ("light", LIGHT), ("last", LIGHT)],
    );
    let times = run(env!("CARGO_BIN_EXE_evolve-time"), "0.02", &given, &taken);
    let ids: Vec<&Value> = times.iter().map(|time| &time["id"]).collect();
    assert_eq!(ids, [&json!("heavy"), &json!("light"), &json!("last")]);
    assert_eq!(times[0]["elapsed_ms"], Value::Null, "{}", times[0]);
    for time in &times {
        assert_eq!(time["running_compiles"], json!(0), "{time}");
    }
}

#[test]
fn a_compile_within_the_budget_is_unaffected() {
    let heavy = inlining(100);
    let (given, taken) = written("time-within", &[("heavy", &heavy), ("light", LIGHT)]);
    let times = run(env!("CARGO_BIN_EXE_evolve-time"), "600", &given, &taken);
    assert_eq!(times.len(), 2);
    for time in &times {
        assert!(time["elapsed_ms"].is_number(), "{time}");
        assert_eq!(time["running_compiles"], json!(0), "{time}");
        assert_eq!(time["codes"], json!(["no-entry-instance"]), "{time}");
    }
}

#[test]
fn the_check_writes_every_verdict_in_order_across_a_compile_past_the_budget() {
    let heavy = inlining(1000);
    let (given, taken) = written(
        "check-past",
        &[("light", LIGHT), ("heavy", &heavy), ("after", LIGHT)],
    );
    let verdicts = run(env!("CARGO_BIN_EXE_evolve-check"), "0.02", &given, &taken);
    let ids: Vec<&Value> = verdicts.iter().map(|verdict| &verdict["id"]).collect();
    assert_eq!(ids, [&json!("light"), &json!("heavy"), &json!("after")]);
    assert_eq!(verdicts[1]["elapsed_ms"], Value::Null, "{}", verdicts[1]);
    assert_eq!(verdicts[1]["signals"], json!(["slow"]), "{}", verdicts[1]);
}
