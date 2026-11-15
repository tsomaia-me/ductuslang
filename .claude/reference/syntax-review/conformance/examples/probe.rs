// Review probe: runs each case of a cases file through `syntax::lex` and `syntax::parse`
// with the SPEC §1.3 example vocabulary and prints diagnostics, tokens and the tree.
//
// Cases file: a line `#### name [flags]` starts a case; the lines up to the next `####`
// are its text. Flags: `esc` translates <TAB> <CR> <BOM> <U+XXXX> and <NOEOL> (drop the
// final line break); `tree` prints the tree; `tokens` prints the significant tokens.
use std::env;
use std::fs;

use symphonyscript::Source;
use symphonyscript::syntax::{ElementView, NodeView, SyntaxKind, lex, parse};

fn unescape(text: &str) -> String {
    let mut out = text
        .replace("<TAB>", "\t")
        .replace("<CR>", "\r")
        .replace("<BOM>", "\u{feff}");
    while let Some(at) = out.find("<U+") {
        let end = out[at..].find('>').map(|e| at + e).unwrap();
        let code = u32::from_str_radix(&out[at + 3..end], 16).unwrap();
        out.replace_range(at..=end, &char::from_u32(code).unwrap().to_string());
    }
    if let Some(at) = out.find("<NOEOL>") {
        out.truncate(at);
    }
    out
}

fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let col = before.rfind('\n').map_or(before, |i| &before[i + 1..]).chars().count() + 1;
    (line, col)
}

fn print_tree(node: &NodeView<'_>, depth: usize, out: &mut String) {
    out.push_str(&"  ".repeat(depth));
    out.push_str(&format!("{:?}@{:?}\n", node.kind(), node.range()));
    for element in node.children_with_tokens() {
        match element {
            ElementView::Node(child) => print_tree(&child, depth + 1, out),
            ElementView::Token(token) => {
                if !token.kind().is_trivia() {
                    out.push_str(&"  ".repeat(depth + 1));
                    out.push_str(&format!("{:?} {:?}\n", token.kind(), token.text()));
                }
            }
        }
    }
}

fn sexpr(node: &NodeView<'_>, out: &mut String) {
    let kind = node.kind();
    let plain = matches!(kind, SyntaxKind::NameRef | SyntaxKind::Literal);
    let bare = matches!(kind, SyntaxKind::BinaryExpr | SyntaxKind::PrefixExpr);
    if !plain {
        out.push('(');
        if !bare {
            out.push_str(&format!("{kind:?} "));
        }
    }
    let mut first = true;
    for element in node.children_with_tokens() {
        match element {
            ElementView::Node(child) => {
                if !first { out.push(' '); }
                first = false;
                sexpr(&child, out);
            }
            ElementView::Token(token) => {
                if token.kind().is_trivia() || matches!(token.kind(), SyntaxKind::Newline | SyntaxKind::Indent | SyntaxKind::Dedent) {
                    continue;
                }
                if !first { out.push(' '); }
                first = false;
                out.push_str(token.text());
            }
        }
    }
    if !plain {
        out.push(')');
    }
}

// Every byte in exactly one token; ranges nest; children in order. Returns faults found.
fn check_tree(node: &NodeView<'_>, faults: &mut Vec<String>, covered: &mut Vec<u32>) {
    let range = node.range();
    let mut at = range.start;
    for element in node.children_with_tokens() {
        let r = element.range();
        if r.start != at {
            faults.push(format!("{:?}@{:?}: child {:?}@{:?} starts at {} not {}", node.kind(), range, element.kind(), r, r.start, at));
        }
        if r.end > range.end || r.start < range.start {
            faults.push(format!("{:?}@{:?}: child {:?}@{:?} outside parent", node.kind(), range, element.kind(), r));
        }
        at = r.end;
        match element {
            ElementView::Node(child) => check_tree(&child, faults, covered),
            ElementView::Token(token) => {
                for b in token.range() {
                    covered[b as usize] += 1;
                }
            }
        }
    }
    if at != range.end {
        faults.push(format!("{:?}@{:?}: children end at {}", node.kind(), range, at));
    }
}

fn run(name: &str, flags: &str, text: &str) {
    let text = if flags.contains("esc") { unescape(text) } else { text.to_string() };
    let source = Source { path: "case.sym".into(), text: text.clone() };
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let lexed = lex(&source, &vocabulary);
    let parsed = parse(&source, &vocabulary);
    println!("#### {name} [{flags}]  lex-diags={} parse-diags={}", lexed.diagnostics.len(), parsed.diagnostics.len());
    for d in &parsed.diagnostics {
        let (line, col) = line_col(&text, d.span.range.start as usize);
        let snippet = text.get(d.span.range.start as usize..d.span.range.end as usize).unwrap_or("?");
        println!("  DIAG {} {}:{} {:?} `{}` :: {}{}", d.code.as_str(), line, col, d.span.range, snippet.escape_debug(), d.summary,
            d.help.as_ref().map(|h| format!(" [help: {h}]")).unwrap_or_default());
    }
    // lexer-only diagnostics that the parse did not carry
    for d in &lexed.diagnostics {
        if !parsed.diagnostics.iter().any(|p| p.code == d.code && p.span == d.span) {
            println!("  LEXONLY {} {:?} :: {}", d.code.as_str(), d.span.range, d.summary);
        }
    }
    // lossless checks
    let mut faults = Vec::new();
    let mut pos = 0u32;
    for token in &lexed.tokens {
        if token.range.start != pos {
            faults.push(format!("lex token {:?} starts at {} not {}", token.kind, token.range.start, pos));
        }
        pos = token.range.end;
    }
    if pos as usize != text.len() {
        faults.push(format!("lex tokens end at {} of {}", pos, text.len()));
    }
    let root = parsed.root();
    if root.to_string() != text {
        faults.push("root text differs from the source".into());
    }
    if root.range() != (0..text.len() as u32) {
        faults.push(format!("root range {:?}", root.range()));
    }
    let mut covered = vec![0u32; text.len()];
    check_tree(&root, &mut faults, &mut covered);
    if let Some(at) = covered.iter().position(|c| *c != 1) {
        faults.push(format!("byte {} is in {} tokens", at, covered[at]));
    }
    for f in &faults {
        println!("  TREEFAULT {f}");
    }
    // tokens of the tree against the tokens of `lex`
    let mut tree_tokens = Vec::new();
    for step in root.preorder() {
        if let symphonyscript::syntax::Walk::Enter(ElementView::Token(token)) = step {
            tree_tokens.push((token.kind(), token.range()));
        }
    }
    let lex_tokens: Vec<_> = lexed.tokens.iter().map(|t| (t.kind, t.range.clone())).collect();
    if tree_tokens != lex_tokens {
        let renamed_only = tree_tokens.len() == lex_tokens.len()
            && tree_tokens.iter().zip(&lex_tokens).all(|(a, b)| a.1 == b.1 && (a.0 == b.0 || (a.0 == SyntaxKind::Ident && b.0.is_keyword())));
        if renamed_only {
            println!("  TOKDIFF keyword-to-ident renames only");
        } else {
            let first = tree_tokens.iter().zip(&lex_tokens).position(|(a, b)| a != b).unwrap_or(tree_tokens.len().min(lex_tokens.len()));
            println!("  TOKDIFF structural: tree has {} tokens, lex has {}; first difference at token {}: tree {:?} lex {:?}", tree_tokens.len(), lex_tokens.len(), first, tree_tokens.get(first), lex_tokens.get(first));
        }
    }
    if flags.contains("tokens") {
        let mut line = String::from("  TOKENS");
        for token in &lexed.tokens {
            if token.kind.is_trivia() { continue; }
            let t = &text[token.range.start as usize..token.range.end as usize];
            line.push_str(&format!(" {:?}{}", token.kind, if matches!(token.kind, SyntaxKind::Newline | SyntaxKind::Indent | SyntaxKind::Dedent) { String::new() } else { format!("({})", t.escape_debug()) }));
        }
        println!("{line}");
    }
    if flags.contains("sexpr") {
        for item in root.children() {
            let mut out = String::new();
            sexpr(&item, &mut out);
            println!("  SEXPR {out}");
        }
    }
    if flags.contains("tree") {
        let mut out = String::new();
        print_tree(&root, 2, &mut out);
        print!("{out}");
    }
}

fn main() {
    let path = env::args().nth(1).expect("a cases file");
    let force = env::args().nth(2).unwrap_or_default();
    let all = fs::read_to_string(&path).expect("the cases file reads");
    let mut name = String::new();
    let mut flags = String::new();
    let mut text = String::new();
    let mut started = false;
    let mut count = 0usize;
    for line in all.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix("#### ") {
            if started {
                run(&name, &format!("{flags} {force}"), &text);
                count += 1;
            }
            started = true;
            let rest = rest.trim_end();
            match rest.split_once(" [") {
                Some((n, f)) => { name = n.to_string(); flags = f.trim_end_matches(']').to_string(); }
                None => { name = rest.to_string(); flags = String::new(); }
            }
            text.clear();
        } else if started {
            text.push_str(line);
        }
    }
    if started {
        run(&name, &format!("{flags} {force}"), &text);
        count += 1;
    }
    println!("CASES {count}");
}
