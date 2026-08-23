//! The `assert_trace` function: byte-compare a trace against its golden.

use std::fmt::Write;

/// Asserts that `actual` is BYTE-identical to `golden`, reporting the first
/// divergent line when it is not.
///
/// Byte equality is the whole comparison: the golden files are byte-compared
/// (R24's schema is a canonical form — one spelling per event), so no
/// normalization happens here. Trimming trailing whitespace or ignoring a
/// final newline would make the harness accept traces the format does not
/// admit.
///
/// `witness` names the witness, so a failure in a file holding a dozen of them
/// says which one broke before the reader looks at anything else.
///
/// # Panics
///
/// On any difference. The report gives the first divergent LINE — its 1-based
/// number, the expected line, the actual line — and then both traces in full,
/// because a trace is read as a sequence of commit blocks and the line in
/// isolation rarely explains itself.
pub fn assert_trace(witness: &str, actual: &str, golden: &str) {
    if actual == golden {
        return;
    }

    panic!("{}", report(witness, actual, golden));
}

/// The failure report: the first divergence, then both traces whole.
fn report(witness: &str, actual: &str, golden: &str) -> String {
    let mut report = format!("witness `{witness}`: trace does not match its golden\n");
    let (line, expected, got) = first_divergence(actual, golden);

    if expected.is_none() && got.is_none() {
        report.push_str(
            "\nevery line agrees: the traces differ in trailing bytes only \
             (a final newline). The comparison is BYTE equality.\n",
        );
    } else {
        let _ = write!(
            report,
            "\nfirst divergence at line {line}:\n  expected: {}\n  actual:   {}\n",
            render(expected),
            render(got)
        );
    }

    let _ = write!(
        report,
        "\n--- expected (golden) ---\n{golden}\
         --- actual ---\n{actual}\
         --- end ---\n"
    );

    report
}

/// The 1-based number of the first differing line, with both sides' text.
///
/// `None` on a side means that side ENDED there — the traces agree as far as
/// the shorter one goes, which is the common shape when a commit block is
/// missing or extra.
fn first_divergence<'t>(
    actual: &'t str,
    golden: &'t str,
) -> (usize, Option<&'t str>, Option<&'t str>) {
    let mut expected_lines = golden.lines();
    let mut actual_lines = actual.lines();
    let mut line = 1;

    loop {
        let expected = expected_lines.next();
        let got = actual_lines.next();

        if expected != got {
            return (line, expected, got);
        }

        // Both ran out together and every line agreed, so the difference is
        // invisible to `lines()` — a missing or extra trailing newline.
        if expected.is_none() {
            return (line, None, None);
        }

        line += 1;
    }
}

/// One side of a divergence, quoted — or named as the end of the trace.
fn render(line: Option<&str>) -> String {
    match line {
        Some(text) => format!("{text:?}"),
        None => "<end of trace>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN: &str =
        "commit_begin\ndirty App.count 0 -> 7\neval App.doubled -> 14\npublish\ncommit_end\n";

    #[test]
    fn accepts_a_byte_identical_trace() {
        assert_trace("single_derived", GOLDEN, GOLDEN);
    }

    #[test]
    #[should_panic(expected = "witness `single_derived`: trace does not match its golden")]
    fn names_the_witness_on_failure() {
        assert_trace("single_derived", "commit_begin\n", GOLDEN);
    }

    // The line NUMBER is the thing a reader uses first: a trace is dozens of
    // near-identical lines, and "line 3" locates the commit block.
    #[test]
    fn reports_the_first_divergent_line_with_both_sides() {
        let actual =
            "commit_begin\ndirty App.count 0 -> 7\neval App.doubled -> 15\npublish\ncommit_end\n";
        let report = report("single_derived", actual, GOLDEN);

        assert!(report.contains("first divergence at line 3"));
        assert!(report.contains("expected: \"eval App.doubled -> 14\""));
        assert!(report.contains("actual:   \"eval App.doubled -> 15\""));
    }

    // A short trace is the shape of a suppressed evaluation or a missing
    // block, so the report says "the trace ended" rather than comparing
    // against an invented empty line.
    #[test]
    fn reports_the_end_of_a_short_trace() {
        let report = report("single_derived", "commit_begin\n", GOLDEN);

        assert!(report.contains("first divergence at line 2"));
        assert!(report.contains("actual:   <end of trace>"));
    }

    #[test]
    fn reports_an_extra_line_past_the_golden() {
        let actual = format!("{GOLDEN}commit_begin\n");
        let report = report("empty_commit", &actual, GOLDEN);

        assert!(report.contains("first divergence at line 6"));
        assert!(report.contains("expected: <end of trace>"));
        assert!(report.contains("actual:   \"commit_begin\""));
    }

    // Byte equality, not line equality: a missing final newline is a real
    // difference, and `lines()` cannot see it — the report still fires, and
    // both traces are printed whole so the reader can spot it.
    #[test]
    #[should_panic(expected = "trace does not match its golden")]
    fn refuses_a_trace_missing_its_final_newline() {
        assert_trace("single_derived", GOLDEN.trim_end(), GOLDEN);
    }

    #[test]
    fn names_a_trailing_byte_difference_for_what_it_is() {
        let report = report("single_derived", GOLDEN.trim_end(), GOLDEN);

        assert!(report.contains("trailing bytes only"));
        assert!(!report.contains("first divergence"));
    }

    #[test]
    fn prints_both_traces_whole() {
        let report = report("single_derived", "commit_begin\n", GOLDEN);

        assert!(report.contains("--- expected (golden) ---"));
        assert!(report.contains("--- actual ---"));
        assert!(report.contains("eval App.doubled -> 14"));
    }
}
