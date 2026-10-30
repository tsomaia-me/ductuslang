use std::process::{Command, Output};
use std::time::Duration;

use symphonyscript_evolve::{
    DEFAULT_TAG, PROBE_TAGS, ProbeProgram, Probed, probe_block, probe_programs, probed,
};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/probe/programs.txt");

fn probe(arguments: &[&str]) -> Output {
    match Command::new(env!("CARGO_BIN_EXE_evolve-probe"))
        .args(arguments)
        .output()
    {
        Ok(output) => output,
        Err(error) => panic!("evolve-probe does not start: {error}"),
    }
}

fn blocks(output: &Output) -> Vec<String> {
    let text = String::from_utf8_lossy(&output.stdout);
    let mut blocks: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.starts_with('[') || line.starts_with("summary:") {
            blocks.push(String::new());
        }
        if let Some(block) = blocks.last_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    blocks
}

fn without_times(block: &str) -> String {
    block
        .lines()
        .filter(|line| !line.ends_with(" ms"))
        .map(|line| format!("{line}\n"))
        .collect()
}

#[test]
fn a_file_splits_into_programs_at_each_separator_line_with_their_tags_and_expectations() {
    let content =
        "\nClip c:\n    Note\n----\n#native\nlet x = 1\n#expect unused-let\n----\n\n----\n#to\n";
    let programs = probe_programs("cases.txt", content);
    let expected = vec![
        ProbeProgram {
            file: "cases.txt".to_string(),
            line: 2,
            tag: DEFAULT_TAG.to_string(),
            text: "Clip c:\n    Note\n".to_string(),
            expect: None,
        },
        ProbeProgram {
            file: "cases.txt".to_string(),
            line: 6,
            tag: "native".to_string(),
            text: "let x = 1\n".to_string(),
            expect: Some("unused-let".to_string()),
        },
        ProbeProgram {
            file: "cases.txt".to_string(),
            line: 11,
            tag: "to".to_string(),
            text: String::new(),
            expect: None,
        },
    ];
    assert_eq!(programs, expected);
}

#[test]
fn an_expectation_with_no_codes_states_a_clean_program() {
    let programs = probe_programs("cases.txt", "Clip c:\n    Note\n#expect\n");
    assert_eq!(
        programs
            .iter()
            .map(|program| program.expect.clone())
            .collect::<Vec<_>>(),
        vec![Some(String::new())]
    );
}

#[test]
fn a_panic_in_the_compiler_is_caught_and_printed_with_its_message() {
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = probed(Duration::from_secs(5), "", || {
        panic!("lowering lost a cell\nat the second line")
    });
    let _ = std::panic::take_hook();
    assert_eq!(
        outcome,
        Probed::Panicked {
            message: "lowering lost a cell\nat the second line".to_string()
        }
    );
    let program = ProbeProgram {
        file: "cases.txt".to_string(),
        line: 1,
        tag: DEFAULT_TAG.to_string(),
        text: String::new(),
        expect: Some("panic".to_string()),
    };
    assert_eq!(
        probe_block(1, &program, &outcome, true),
        "[1] #example cases.txt:1\npanic lowering lost a cell at the second line\nok\n"
    );
}

#[test]
fn a_clean_program_prints_clean_and_its_compile_time() {
    let output = probe(&[FIXTURE]);
    assert!(output.status.success());
    let blocks = blocks(&output);
    let lines: Vec<&str> = blocks[0].lines().collect();
    assert_eq!(
        lines[..2],
        [format!("[1] #example {FIXTURE}:1"), "clean".to_string()]
    );
    let elapsed = lines[2].strip_suffix(" ms").map(str::parse::<f64>);
    assert!(matches!(elapsed, Some(Ok(ms)) if ms >= 0.0), "{}", lines[2]);
    assert_eq!(lines.len(), 3);
}

#[test]
fn an_error_prints_its_code_span_summary_help_and_related_spans() {
    let output = probe(&[FIXTURE]);
    assert_eq!(
        without_times(&blocks(&output)[1]),
        format!(
            "[2] #example {FIXTURE}:10\nduplicate-name 2:5-2:6 `x` is declared twice\nhelp: rename one of the two declarations\n  1:5-1:6 first declared here\n"
        )
    );
}

#[test]
fn a_tag_chooses_the_vocabulary_the_program_compiles_against() {
    let output = probe(&[FIXTURE]);
    let blocks: Vec<String> = blocks(&output)
        .iter()
        .map(|block| without_times(block))
        .collect();
    assert_eq!(blocks[2], format!("[3] #signature {FIXTURE}:22\nclean\n"));
    assert!(
        blocks[3].starts_with(&format!("[4] #example {FIXTURE}:26\n")),
        "{}",
        blocks[3]
    );
    assert!(
        blocks[3].contains("\nunknown-attribute 1:10-1:15 `Clip` has no attribute `three`\n"),
        "{}",
        blocks[3]
    );
    assert!(
        blocks[4].starts_with(&format!("[5] #native {FIXTURE}:31\nno-entry-instance ")),
        "{}",
        blocks[4]
    );
    assert_eq!(blocks[5], format!("[6] #to {FIXTURE}:36\nclean\n"));
}

#[test]
fn values_compare_each_expectation_with_the_codes_drawn_and_the_summary_counts_them() {
    let output = probe(&["--values", "--summary", FIXTURE]);
    let blocks = blocks(&output);
    let verdicts: Vec<&str> = blocks
        .iter()
        .map(|block| {
            block
                .lines()
                .last()
                .filter(|line| !line.ends_with(" ms"))
                .unwrap_or("no verdict")
        })
        .collect();
    assert_eq!(
        verdicts,
        [
            "ok",
            "ok",
            "ok",
            "differs: expected ``, drew `no-entry-instance unknown-attribute`",
            "ok",
            "no verdict",
            "summary: programs 6, clean 3, errors 2, warnings 2, panics 0, timeouts 0, ok 4, differs 1",
        ]
    );
}

#[test]
fn without_values_no_expectation_is_judged() {
    let output = probe(&["--summary", FIXTURE]);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !text
            .lines()
            .any(|line| line == "ok" || line.starts_with("differs")),
        "{text}"
    );
    assert!(
        text.ends_with(
            "summary: programs 6, clean 3, errors 2, warnings 2, panics 0, timeouts 0\n"
        ),
        "{text}"
    );
}

#[test]
fn a_program_past_the_budget_times_out_and_the_run_goes_on() {
    let output = probe(&["--budget", "0", "--summary", FIXTURE]);
    assert!(output.status.success());
    let blocks = blocks(&output);
    assert_eq!(blocks.len(), 7);
    for block in &blocks[..6] {
        assert_eq!(block.lines().nth(1), Some("timeout"), "{block}");
    }
    assert_eq!(
        blocks[6],
        "summary: programs 6, clean 0, errors 0, warnings 0, panics 0, timeouts 6\n"
    );
}

#[test]
fn programs_run_in_parallel_print_in_file_order() {
    let one: Vec<String> = blocks(&probe(&["--jobs", "1", FIXTURE, FIXTURE]))
        .iter()
        .map(|block| without_times(block))
        .collect();
    let many: Vec<String> = blocks(&probe(&["--jobs", "4", FIXTURE, FIXTURE]))
        .iter()
        .map(|block| without_times(block))
        .collect();
    assert_eq!(one.len(), 12);
    assert_eq!(one, many);
}

#[test]
fn the_tags_print_the_one_tag_table_and_every_tag_in_it_compiles_under_its_vocabulary() {
    let output = probe(&["--tags"]);
    assert!(output.status.success());
    let printed: Vec<(String, String)> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| match line.split_once(' ') {
            Some((tag, vocabulary)) => (tag.to_string(), vocabulary.to_string()),
            None => panic!("a tag line without a vocabulary: `{line}`"),
        })
        .collect();
    let table: Vec<(String, String)> = PROBE_TAGS
        .iter()
        .map(|(tag, vocabulary)| (tag.to_string(), vocabulary.to_string()))
        .collect();
    assert_eq!(printed, table);
    let file = concat!(env!("CARGO_TARGET_TMPDIR"), "/every-tag.txt");
    let content: Vec<String> = PROBE_TAGS
        .iter()
        .map(|(tag, _)| format!("#{tag}\nClip c:\n    Note\n"))
        .collect();
    if let Err(error) = std::fs::write(file, content.join("----\n")) {
        panic!("{file}: {error}");
    }
    let output = probe(&[file]);
    assert!(output.status.success());
    let blocks = blocks(&output);
    assert_eq!(blocks.len(), PROBE_TAGS.len());
    for ((tag, _), block) in PROBE_TAGS.iter().zip(&blocks) {
        let mut lines = block.lines();
        let header = lines.next().unwrap_or_default();
        assert!(header.contains(&format!(" #{tag} ")), "{block}");
        assert!(
            !lines.next().unwrap_or_default().starts_with("panic"),
            "{block}"
        );
    }
}

#[test]
fn an_unknown_tag_stops_the_run_before_any_program_compiles() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/probe/unknown-tag.txt");
    let output = probe(&[file]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "evolve-probe: {file}:2: unknown tag `#vocab /a/vocabulary.rs`; the tags are #example, #native, #bounded, #envelope, #bar, #signature, #to\n"
        )
    );
}
