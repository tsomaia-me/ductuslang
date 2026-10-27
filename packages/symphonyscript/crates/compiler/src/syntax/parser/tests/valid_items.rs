use std::collections::BTreeSet;
use std::ops::Range;

use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{dsp_project, music_project, pending_spec_examples, spec_examples};

use crate::source::Source;
use crate::syntax::{NodeView, SyntaxKind, parse};

pub(super) const HOST: &str = "Clip host:\n";

const BODY_COLUMN: usize = 4;

const BODY_ITEMS: &[SyntaxKind] = &[
    SyntaxKind::Placement,
    SyntaxKind::WirePlacement,
    SyntaxKind::Line,
    SyntaxKind::ForBlock,
    SyntaxKind::WhenBlock,
];

const ADDITION_ITEMS: &[&str] = &[
    "let u = u8(3) + 2u8\n",
    "input k: u8[..4] = []\n",
    "fn f(i) bounded 2 else 0: f(i + 1)\n",
    "let s = [1, 2, 3][^2..][..1]\n",
    "state[2] s = s{2, 0} + 1\n",
];

pub(super) struct ValidItems {
    pub(super) top: Vec<String>,
    pub(super) body: Vec<String>,
}

pub(super) fn valid_items(vocabulary: &Vocabulary) -> ValidItems {
    let pending = pending_spec_examples();
    let examples = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym" && !pending.contains(&example.name()))
        .map(|example| example.text);
    let files = [music_project(), dsp_project()]
        .into_iter()
        .flat_map(|project| project.files)
        .map(|file| file.text);
    let mut top = BTreeSet::new();
    let mut body = BTreeSet::new();
    let additions = ADDITION_ITEMS.iter().map(|item| item.to_string());
    for text in examples.chain(files).chain(additions) {
        let parsed = parse(&source(&text), vocabulary);
        assert_eq!(parsed.diagnostics, vec![], "{text:?}");
        for item in parsed.root().children() {
            top.insert(lines_of(&text, item.range()));
            for line in body_lines(&item) {
                if let Some(line) = reindented(&lines_of(&text, line.range()), &text, line.range())
                {
                    body.insert(line);
                }
            }
        }
    }
    for item in &top {
        assert_eq!(
            parse(&source(item), vocabulary).diagnostics,
            vec![],
            "{item:?}"
        );
    }
    for item in &body {
        let hosted = format!("{HOST}{item}");
        assert_eq!(
            parse(&source(&hosted), vocabulary).diagnostics,
            vec![],
            "{hosted:?}"
        );
    }
    ValidItems {
        top: top.into_iter().collect(),
        body: body.into_iter().collect(),
    }
}

fn source(text: &str) -> Source {
    Source {
        path: "item.sym".into(),
        text: text.into(),
    }
}

fn body_lines<'p>(item: &NodeView<'p>) -> Vec<NodeView<'p>> {
    let block = match item.kind() {
        SyntaxKind::Placement => item
            .children()
            .find(|child| child.kind() == SyntaxKind::Body)
            .and_then(|body| {
                body.children()
                    .find(|child| child.kind() == SyntaxKind::Block)
            }),
        SyntaxKind::PatternDecl => item
            .children()
            .find(|child| child.kind() == SyntaxKind::Block),
        _ => None,
    };
    block
        .into_iter()
        .flat_map(|block| block.children())
        .filter(|line| BODY_ITEMS.contains(&line.kind()))
        .collect()
}

fn lines_of(text: &str, range: Range<u32>) -> String {
    let start = text[..range.start as usize]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let inside = &text[range.start as usize..range.end as usize];
    let last = range.start as usize + inside.trim_end().len();
    let end = text[last..]
        .find('\n')
        .map_or(text.len(), |offset| last + offset + 1);
    let mut lines = text[start..end].to_string();
    if !lines.ends_with('\n') {
        lines.push('\n');
    }
    lines
}

fn reindented(lines: &str, text: &str, range: Range<u32>) -> Option<String> {
    let line_start = text[..range.start as usize]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let column = range.start as usize - line_start;
    let mut out = String::new();
    for line in lines.split_inclusive('\n') {
        let indentation = line.len() - line.trim_start_matches(' ').len();
        if line.trim().is_empty() {
            out.push_str(line);
        } else if column > BODY_COLUMN {
            let removed = column - BODY_COLUMN;
            if indentation < removed {
                return None;
            }
            out.push_str(&line[removed..]);
        } else {
            out.push_str(&" ".repeat(BODY_COLUMN - column));
            out.push_str(line);
        }
    }
    Some(out)
}
