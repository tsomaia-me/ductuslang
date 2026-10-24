mod support;

use symphonyscript::syntax::{ElementView, SyntaxKind, Walk};

#[test]
fn views_navigate_children_parents_and_ancestors() {
    let text = "Clip c:\n    Note/60\n";
    let parsed = support::parsed(text);
    let root = parsed.root();
    assert_eq!(root.kind(), SyntaxKind::File);
    assert_eq!(root.range(), 0..text.len() as u32);
    assert_eq!(root.text(), text);
    assert!(root.parent().is_none());
    let placement = root.children().next().expect("a placement");
    assert_eq!(placement.kind(), SyntaxKind::Placement);
    assert_eq!(placement.parent(), Some(root.clone()));
    let literal = root
        .preorder()
        .filter_map(|event| match event {
            Walk::Enter(ElementView::Node(node)) => Some(node),
            _ => None,
        })
        .find(|node| node.kind() == SyntaxKind::Literal)
        .expect("a literal");
    assert_eq!(literal.text(), "60");
    let kinds: Vec<SyntaxKind> = literal.ancestors().map(|node| node.kind()).collect();
    assert_eq!(
        kinds,
        [
            SyntaxKind::Literal,
            SyntaxKind::DefaultArg,
            SyntaxKind::Placement,
            SyntaxKind::Block,
            SyntaxKind::Body,
            SyntaxKind::Placement,
            SyntaxKind::File,
        ]
    );
    let elements: Vec<SyntaxKind> = placement
        .children_with_tokens()
        .map(|element| element.kind())
        .collect();
    assert_eq!(
        elements,
        [
            SyntaxKind::Ident,
            SyntaxKind::Whitespace,
            SyntaxKind::Ident,
            SyntaxKind::Body
        ]
    );
}

#[test]
fn views_walk_in_preorder_and_find_tokens_by_offset() {
    let text = "let a = 1\n";
    let parsed = support::parsed(text);
    let root = parsed.root();
    let mut entered = 0;
    let mut left = 0;
    let mut rebuilt = String::new();
    for event in root.preorder() {
        match event {
            Walk::Enter(ElementView::Token(token)) => {
                rebuilt.push_str(token.text());
                entered += 1;
            }
            Walk::Enter(ElementView::Node(_)) => entered += 1,
            Walk::Leave(_) => left += 1,
        }
    }
    assert_eq!(rebuilt, text);
    assert_eq!(entered, left);
    let inside = root.token_at_offset(1);
    assert_eq!(inside.len(), 1);
    assert_eq!(inside[0].text(), "let");
    assert_eq!(inside[0].range(), 0..3);
    let texts = |offset: u32| -> Vec<String> {
        root.token_at_offset(offset)
            .iter()
            .map(|token| token.text().to_string())
            .collect()
    };
    assert_eq!(texts(4), [" ", "a"]);
    assert_eq!(texts(5), ["a", " "]);
    assert_eq!(texts(0), ["let"]);
    assert!(root.token_at_offset(99).is_empty());
    let first = root.first_token().expect("a first token");
    assert_eq!(first.kind(), SyntaxKind::LetKw);
    assert_eq!(
        first.next_token().map(|token| token.kind()),
        Some(SyntaxKind::Whitespace)
    );
    assert_eq!(
        root.last_token().map(|token| token.kind()),
        Some(SyntaxKind::Newline)
    );
    assert_eq!(
        first.parent().map(|node| node.kind()),
        Some(SyntaxKind::LetDecl)
    );
    assert!(first.prev_token().is_none());
    assert_eq!(format!("{first}"), "let");
}
