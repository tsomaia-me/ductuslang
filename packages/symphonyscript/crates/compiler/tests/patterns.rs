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
