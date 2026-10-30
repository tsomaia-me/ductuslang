mod support;

use std::collections::BTreeSet;

use symphonyscript::Source;
use symphonyscript::syntax::{SyntaxKind, lex};
use symphonyscript_fixtures::{
    dsp_project, music_project, pending_spec_examples, spec_examples, vocabulary,
};

const BRACKETS: [SyntaxKind; 8] = [
    SyntaxKind::Bundle,
    SyntaxKind::History,
    SyntaxKind::Conversion,
    SyntaxKind::ParenPlacement,
    SyntaxKind::ArrayLit,
    SyntaxKind::ArgList,
    SyntaxKind::IndexExpr,
    SyntaxKind::ParenExpr,
];

const LOOP_FORMS: &str = "input held: i32[..10] = []\nstate total = 1 + (for i: i32, x in held into sum: i32 from 0: sum + i * x)\nClip c:\n    [Note/60 for i, p: i32 in [60, 64]: Note/p]\n    Note/(for x in [1, 2] into s from 0: s + x)\n";

const HISTORY_READ: &str = "input note = 60\nstate recent = (recent{1, []} + [note])[^8..]\nClip c:\n    Note/(recent[^1])\n";

fn valid_sources() -> Vec<String> {
    let pending = pending_spec_examples();
    let examples = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym" && !pending.contains(&example.name()))
        .map(|example| example.text);
    let files = [music_project(), dsp_project()]
        .into_iter()
        .flat_map(|project| project.files)
        .map(|file| file.text);
    examples
        .chain(files)
        .chain([HISTORY_READ.to_string(), LOOP_FORMS.to_string()])
        .collect()
}

fn indentation_of_line_at(text: &str, offset: usize) -> usize {
    let start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    text[start..].len() - text[start..].trim_start_matches(' ').len()
}

fn break_points_in_brackets(text: &str) -> Vec<(usize, usize, usize)> {
    let source = Source {
        path: "variant.sym".into(),
        text: text.into(),
    };
    let mut openers = Vec::new();
    let mut strings = 0usize;
    let mut found = Vec::new();
    for token in lex(&source, &vocabulary()).tokens {
        let start = token.range.start as usize;
        let end = token.range.end as usize;
        match token.kind {
            SyntaxKind::StringStart => strings += 1,
            SyntaxKind::StringEnd => strings = strings.saturating_sub(1),
            _ if strings > 0 => {}
            SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::LBrace => {
                openers.push(start);
                found.push((end, end, start));
            }
            SyntaxKind::RParen | SyntaxKind::RBracket | SyntaxKind::RBrace => {
                if let Some(opener) = openers.pop() {
                    found.push((start, start, opener));
                }
            }
            SyntaxKind::Whitespace | SyntaxKind::Comma => {
                if let Some(opener) = openers.last() {
                    let point = if token.kind == SyntaxKind::Comma {
                        (end, end)
                    } else {
                        (start, end)
                    };
                    found.push((point.0, point.1, *opener));
                }
            }
            _ => {}
        }
    }
    found
}

fn innermost_bracket(text: &str, offset: usize) -> Option<SyntaxKind> {
    let parsed = support::parsed(text);
    let token = parsed
        .root()
        .token_at_offset(offset as u32)
        .into_iter()
        .next()?;
    token
        .parent()?
        .ancestors()
        .map(|node| node.kind())
        .find(|kind| BRACKETS.contains(kind))
}

#[test]
fn line_breaks_inside_brackets_change_no_valid_program() {
    let mut covered = BTreeSet::new();
    let mut variants = 0;
    for text in valid_sources() {
        let original = support::tree(&text);
        for (start, end, opener) in break_points_in_brackets(&text) {
            if let Some(kind) = innermost_bracket(&text, start) {
                covered.insert(kind);
            }
            let column = indentation_of_line_at(&text, opener);
            for spaces in [0, column, column + 1] {
                let variant = format!("{}\n{}{}", &text[..start], " ".repeat(spaces), &text[end..]);
                assert_eq!(support::codes(&variant), vec![], "{variant:?}");
                assert_eq!(support::tree(&variant), original, "{variant:?}");
                variants += 1;
            }
        }
    }
    assert!(variants > 0);
    assert_eq!(covered, BTreeSet::from(BRACKETS), "{variants} variants");
}
