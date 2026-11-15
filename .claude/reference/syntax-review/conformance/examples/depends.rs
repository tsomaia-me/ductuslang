// Review probe: what a parse result depends on. Parses every case twice, under another
// path, and against an empty vocabulary, and reports what changes.
use std::env;
use std::fs;

use symphonyscript::Source;
use symphonyscript::syntax::{ElementView, SyntaxKind, Walk, parse};
use symphonyscript_core::Vocabulary;

fn shape(parsed: &symphonyscript::syntax::Parsed) -> (Vec<(SyntaxKind, u32, u32)>, Vec<(String, u32, u32, String)>) {
    let mut nodes = Vec::new();
    for step in parsed.root().preorder() {
        if let Walk::Enter(element) = step {
            let r = element.range();
            let token = matches!(element, ElementView::Token(_));
            let _ = token;
            nodes.push((element.kind(), r.start, r.end));
        }
    }
    let diags = parsed.diagnostics.iter().map(|d| (d.code.as_str().to_string(), d.span.range.start, d.span.range.end, d.summary.clone())).collect();
    (nodes, diags)
}

fn main() {
    let empty = Vocabulary::builder("empty").build().expect("an empty vocabulary is valid");
    let example = symphonyscript_fixtures::vocabulary();
    let (mut total, mut repeat_differs, mut path_differs, mut vocab_tree, mut vocab_diag) = (0, 0, 0, 0, 0);
    for path in env::args().skip(1) {
        let all = fs::read_to_string(&path).expect("the cases file reads");
        let mut texts: Vec<String> = Vec::new();
        for line in all.split_inclusive('\n') {
            if line.starts_with("#### ") { texts.push(String::new()); } else if let Some(last) = texts.last_mut() { last.push_str(line); }
        }
        for text in texts {
            total += 1;
            let a = parse(&Source { path: "a.sym".into(), text: text.clone() }, &example);
            let b = parse(&Source { path: "a.sym".into(), text: text.clone() }, &example);
            if a != b { repeat_differs += 1; }
            let c = parse(&Source { path: "dir/other.sym".into(), text: text.clone() }, &example);
            if shape(&a) != shape(&c) { path_differs += 1; }
            let d = parse(&Source { path: "a.sym".into(), text: text.clone() }, &empty);
            let (sa, sd) = (shape(&a), shape(&d));
            if sa.0 != sd.0 { vocab_tree += 1; }
            if sa.1 != sd.1 { vocab_diag += 1; }
        }
    }
    println!("programs={total} repeat-parse-differs={repeat_differs} other-path-differs(tree or diagnostics, file name apart)={path_differs} empty-vocabulary: tree-differs={vocab_tree} diagnostics-differ={vocab_diag}");
}
