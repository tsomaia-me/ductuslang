use std::fmt::Write;

use crate::probe_program::ProbeProgram;
use crate::probed::{PROBED_PATH, Probed, ProbedSpan};

pub fn probe_block(index: usize, program: &ProbeProgram, probed: &Probed, values: bool) -> String {
    let mut block = format!(
        "[{index}] #{} {}:{}\n",
        program.tag, program.file, program.line
    );
    match probed {
        Probed::Finished {
            elapsed_ms,
            diagnostics,
        } => {
            if diagnostics.is_empty() {
                block.push_str("clean\n");
            }
            for diagnostic in diagnostics {
                let summary = diagnostic.summary.lines().next().unwrap_or_default();
                let _ = writeln!(
                    block,
                    "{} {} {summary}",
                    diagnostic.code,
                    spanned(&diagnostic.span)
                );
                if let Some(help) = &diagnostic.help {
                    let _ = writeln!(block, "help: {}", help.replace('\n', " "));
                }
                for related in &diagnostic.related {
                    let _ = writeln!(block, "  {} {}", spanned(&related.span), related.label);
                }
            }
            let _ = writeln!(block, "{elapsed_ms:.1} ms");
        }
        Probed::Panicked { message } => {
            let _ = writeln!(block, "panic {}", message.replace('\n', " "));
        }
        Probed::TimedOut => block.push_str("timeout\n"),
    }
    if values && let Some(expected) = &program.expect {
        let drawn = drawn_codes(probed);
        if *expected == drawn {
            block.push_str("ok\n");
        } else {
            let _ = writeln!(block, "differs: expected `{expected}`, drew `{drawn}`");
        }
    }
    block
}

pub fn drawn_codes(probed: &Probed) -> String {
    match probed {
        Probed::Finished { diagnostics, .. } => diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        Probed::Panicked { .. } => "panic".to_string(),
        Probed::TimedOut => "timeout".to_string(),
    }
}

fn spanned(span: &ProbedSpan) -> String {
    let [start_line, start_column] = span.start;
    let [end_line, end_column] = span.end;
    let file = if span.file == PROBED_PATH {
        String::new()
    } else {
        format!("{}:", span.file)
    };
    format!("{file}{start_line}:{start_column}-{end_line}:{end_column}")
}
