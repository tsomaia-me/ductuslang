mod support;

use symphonyscript::syntax::lex;
use symphonyscript_fixtures::vocabulary;

const DEPTH: usize = 100_000;
const BODY_DEPTH: usize = 3_000;

fn on_small_stack(work: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(work)
        .expect("the thread starts")
        .join()
        .expect("the work finishes without overflowing");
}

fn round_trip(text: String, clean: bool) {
    let lexed = lex(&support::source(&text), &vocabulary());
    assert!(lexed.tokens.len() > DEPTH.min(BODY_DEPTH));
    drop(lexed);
    let parsed = support::parsed(&text);
    assert_eq!(
        parsed.diagnostics.is_empty(),
        clean,
        "{:?}",
        parsed.diagnostics.first()
    );
    let tree = parsed.root();
    assert_eq!(tree.to_string(), text);
    assert!(support::sexp(&tree).starts_with("File["));
    drop(tree);
    let again = support::parsed(&text);
    assert_eq!(parsed, again);
    assert!(format!("{again:?}").starts_with("Parsed {"));
    let copy = parsed.clone();
    drop(again);
    drop(parsed);
    drop(copy);
}

fn nested(open: &str, middle: &str, close: &str) -> String {
    format!("{}{middle}{}", open.repeat(DEPTH), close.repeat(DEPTH))
}

#[test]
fn sources_nested_in_parentheses_brackets_and_arrays_round_trip() {
    on_small_stack(|| {
        let text = format!(
            "let a = {}\nlet b = {}\nlet c = {}\nlet d = {}\n",
            nested("(", "1", ")"),
            nested("[", "1", "]"),
            nested("x[", "0", "]"),
            nested("([", "1", "])"),
        );
        round_trip(text, true);
    });
}

#[test]
fn sources_nested_in_operators_calls_strings_and_defaults_round_trip() {
    on_small_stack(|| {
        let text = format!(
            "let a = {}1\nlet b = {}x\nlet c = {}\nlet d = {}\nlet e = 2{}\nClip c:\n    Note/{}\n",
            "-".repeat(DEPTH),
            "not ".repeat(DEPTH),
            nested("f(", "1", ")"),
            nested("\"{", "1", "}\""),
            " ** 2".repeat(DEPTH),
            nested("(", "60", ")"),
        );
        round_trip(text, true);
    });
}

#[test]
fn sources_nested_in_bundles_and_parenthesized_placements_round_trip() {
    on_small_stack(|| {
        round_trip(
            format!("Clip c:\n    {}\n", nested("[", "Note", "]")),
            false,
        );
        round_trip(
            format!("Clip c:\n    {}\n", nested("(", "Note", ")")),
            false,
        );
    });
}

#[test]
fn sources_nested_in_bodies_round_trip() {
    on_small_stack(|| {
        let mut text = String::from("Clip top:\n");
        let mut indent = 1;
        for level in 0..BODY_DEPTH {
            let pad = " ".repeat(indent);
            match level % 4 {
                0 => text.push_str(&format!("{pad}Note:\n")),
                1 => text.push_str(&format!("{pad}for i in 0..1:\n")),
                2 => text.push_str(&format!("{pad}when x:\n")),
                _ => {
                    text.push_str(&format!("{pad}when:\n{pad} x:\n"));
                    indent += 1;
                }
            }
            indent += 1;
        }
        text.push_str(&format!("{}Note\n", " ".repeat(indent)));
        round_trip(text, true);
    });
}

#[test]
fn the_typed_view_walks_a_deep_tree() {
    use symphonyscript::syntax::ast::{Expr, Item};
    on_small_stack(|| {
        let text = format!("let a = {}\n", nested("(", "1", ")"));
        let parsed = support::parsed(&text);
        let file = parsed.file().expect("a file");
        let Some(Item::Let(decl)) = file.items().next() else {
            panic!("a let");
        };
        let mut value = decl.value().expect("a value");
        let mut depth = 0;
        while let Expr::Paren(paren) = value {
            value = paren.inner().expect("an inner value");
            depth += 1;
        }
        assert_eq!(depth, DEPTH);
        assert!(matches!(value, Expr::Literal(_)));
        drop(value);
        drop(decl);
        drop(file);
        drop(parsed);
    });
}

#[test]
fn views_of_a_deep_parse_drop_before_their_parse_result() {
    use symphonyscript::syntax::{ElementView, NodeView, Walk};
    on_small_stack(|| {
        let text = format!("let a = {}\n", nested("[", "1", "]"));
        let parsed = support::parsed(&text);
        let root = parsed.root();
        let nodes: Vec<NodeView<'_>> = root
            .preorder()
            .filter_map(|event| match event {
                Walk::Enter(ElementView::Node(node)) => Some(node),
                _ => None,
            })
            .collect();
        assert!(nodes.len() > DEPTH);
        let deepest = nodes.last().expect("a deepest node").clone();
        assert_eq!(deepest.ancestors().count(), DEPTH + 3);
        let offset = deepest.range().start;
        let tokens = root.token_at_offset(offset);
        assert!(!tokens.is_empty());
        let first = root.first_token().expect("a first token");
        let last = root.last_token().expect("a last token");
        let inner = deepest.first_token().expect("the innermost token");
        assert_eq!(inner.text(), "1");
        assert_eq!(
            inner
                .prev_token()
                .map(|token| token.text().to_string())
                .as_deref(),
            Some("[")
        );
        assert_eq!(
            inner
                .next_token()
                .map(|token| token.text().to_string())
                .as_deref(),
            Some("]")
        );
        assert_eq!(first.text(), "let");
        assert_eq!(last.text(), "\n");
        assert_eq!(
            deepest
                .last_token()
                .map(|token| token.text().to_string())
                .as_deref(),
            Some("1")
        );
        drop((first, last, inner));
        let parent = deepest.parent().expect("a parent");
        drop(parent);
        drop(tokens);
        drop(deepest);
        drop(nodes);
        drop(root);
        drop(parsed);
    });
}
