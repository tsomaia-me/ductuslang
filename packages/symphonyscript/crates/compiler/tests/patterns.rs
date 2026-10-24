mod support;

use symphonyscript::Code;

#[test]
fn a_diagnostic_in_an_expanded_pattern_points_at_the_pattern_line_and_every_use() {
    let text = "pattern Inner(default root):\n    Note/root | loud=1\npattern Outer(default root):\n    Inner/root\nClip c:\n    Outer/60\n";
    let diagnostics = support::checked(text);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.code, Code::UnknownAttribute);
    let (start, end) = support::at(text, "loud", 0);
    assert_eq!(diagnostic.span.range, start..end);
    let inner = support::at(text, "Inner/root", 0);
    let outer = support::at(text, "Outer/60", 0);
    let secondary: Vec<_> = diagnostic
        .secondary
        .iter()
        .map(|span| span.range.clone())
        .collect();
    assert_eq!(secondary, vec![inner.0..inner.1, outer.0..outer.1]);
}

#[test]
fn a_vocabulary_check_on_an_expanded_placement_points_at_the_pattern_line_and_the_use() {
    let text = "pattern Voice():\n    Osc\nClip c:\n    Voice\n";
    let diagnostics = support::checked(text);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, Code::ChildNotAccepted);
    let (start, end) = support::at(text, "Osc", 0);
    assert_eq!(diagnostics[0].span.range, start..end);
    let (use_start, _) = support::at(text, "Voice\n", 0);
    assert_eq!(
        diagnostics[0]
            .secondary
            .iter()
            .map(|span| span.range.clone())
            .collect::<Vec<_>>(),
        vec![use_start..use_start + 5]
    );
}

#[test]
fn each_use_of_a_pattern_reports_its_own_expansion() {
    let text = "pattern Voice(default root):\n    Note/root\nClip c:\n    Voice/60\n    Voice/3000000000\n    Voice/4000000000\n";
    let diagnostics = support::checked(text);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        vec![Code::DoesNotFit, Code::DoesNotFit],
        "{diagnostics:?}"
    );
    let (root, root_end) = support::at(text, "root\n", 0);
    for (diagnostic, written) in diagnostics
        .iter()
        .zip(["Voice/3000000000", "Voice/4000000000"])
    {
        assert_eq!(diagnostic.span.range, root..root_end - 1);
        let (start, end) = support::at(text, written, 0);
        assert_eq!(
            diagnostic
                .secondary
                .iter()
                .map(|span| span.range.clone())
                .collect::<Vec<_>>(),
            vec![start..end]
        );
    }
}

fn error_codes(text: &str) -> Vec<Code> {
    support::checked(text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn a_bundle_a_pattern_supplies_inside_a_bundle_is_no_member() {
    let both = "pattern P():\n    Note/60\n    [Note/62 Note/64]\nClip c:\n    [P]\n";
    let diagnostics = support::checked(both);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>(),
        vec![Code::BundleMember],
        "{diagnostics:?}"
    );
    let (start, end) = support::at(both, "[Note/62 Note/64]", 0);
    assert_eq!(diagnostics[0].span.range, start..end);
    let (use_start, _) = support::at(both, "[P]", 0);
    assert_eq!(
        diagnostics[0]
            .secondary
            .iter()
            .map(|span| span.range.clone())
            .collect::<Vec<_>>(),
        vec![use_start + 1..use_start + 2]
    );
    for (text, expected) in [
        (
            "pattern P():\n    [Note/62 Note/64]\nClip c:\n    [P]\n",
            vec![Code::BundleMember],
        ),
        (
            "pattern P():\n    Note/60\n    Note/64\nClip c:\n    [P]\n",
            vec![],
        ),
        (
            "pattern P():\n    for i in 0..2:\n        Note/(60 + i)\nClip c:\n    [P]\n",
            vec![],
        ),
        (
            "input on = true\npattern P():\n    when on:\n        Note/60\n    else:\n        Note/62\nClip c:\n    [P]\n",
            vec![],
        ),
    ] {
        assert_eq!(error_codes(text), expected, "{text}");
    }
}

#[test]
fn the_placement_a_stand_in_chain_creates_in_a_bundle_has_no_body() {
    for (text, expected) in [
        (
            "pattern Inner():\n    Synth:\n        Note\npattern Outer():\n    Inner\nTrack t:\n    [Outer]\nSession s:\n    Includes: t\n",
            vec![Code::BundleMember],
        ),
        (
            "pattern Inner():\n    Synth:\n        Note\nTrack t:\n    [Inner]\nSession s:\n    Includes: t\n",
            vec![Code::BundleMember],
        ),
        (
            "pattern Inner():\n    Synth\npattern Outer():\n    Inner\nTrack t:\n    [Outer]\nSession s:\n    Includes: t\n",
            vec![],
        ),
    ] {
        assert_eq!(error_codes(text), expected, "{text}");
    }
}
