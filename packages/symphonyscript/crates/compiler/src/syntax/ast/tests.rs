use super::{AstNode, BodyItem, Expr, Item};
use crate::syntax::token_columns::TokenColumns;
use crate::syntax::tree_columns::TreeColumns;
use crate::syntax::{Parsed, SyntaxKind};

fn tree(shape: &[(Option<SyntaxKind>, SyntaxKind, &str)]) -> Parsed {
    let mut text = String::new();
    let mut tokens = TokenColumns::default();
    let mut tree = TreeColumns::default();
    let mut open: Vec<(SyntaxKind, u32, u32)> = Vec::new();
    let count = |tokens: &TokenColumns| u32::try_from(tokens.len()).unwrap_or(u32::MAX);
    for (opens, kind, piece) in shape {
        match opens {
            Some(SyntaxKind::Dedent) => {
                if let Some((kind, first, nodes)) = open.pop() {
                    tree.push(kind, (first, count(&tokens)), nodes);
                }
            }
            Some(_) => open.push((
                *kind,
                count(&tokens),
                u32::try_from(tree.len()).unwrap_or(u32::MAX),
            )),
            None => {
                if !kind.is_trivia() {
                    tokens.sig.push(count(&tokens));
                }
                tokens.kinds.push(*kind);
                tokens
                    .starts
                    .push(u32::try_from(text.len()).unwrap_or(u32::MAX));
                text.push_str(piece);
            }
        }
    }
    tokens
        .starts
        .push(u32::try_from(text.len()).unwrap_or(u32::MAX));
    Parsed::new(text, tokens, tree, Vec::new(), Vec::new())
}

#[test]
fn the_typed_view_reads_a_hand_built_tree() {
    use SyntaxKind::*;
    let start = |kind| (Some(Indent), kind, "");
    let end = (Some(Dedent), Error, "");
    let token = |kind, text| (None, kind, text);
    let parsed = tree(&[
        start(File),
        start(Placement),
        token(Ident, "Clip"),
        token(Whitespace, " "),
        token(Ident, "Cafe\u{301}"),
        start(Body),
        token(Colon, ":"),
        start(Block),
        token(Newline, "\n"),
        token(Whitespace, "    "),
        token(Indent, ""),
        start(Placement),
        token(Ident, "Note"),
        start(DefaultArg),
        token(Slash, "/"),
        start(Literal),
        token(Number, "60"),
        end,
        end,
        token(Newline, "\n"),
        end,
        token(Dedent, ""),
        end,
        end,
        end,
        end,
    ]);
    assert_eq!(
        parsed.root().to_string(),
        "Clip Cafe\u{301}:\n    Note/60\n"
    );
    let file = super::File::cast(parsed.root()).expect("a file");
    let items: Vec<Item> = file.items().collect();
    let Some(Item::Placement(clip)) = items.first() else {
        panic!("a placement");
    };
    assert_eq!(clip.kind().map(|name| name.text()).as_deref(), Some("Clip"));
    assert_eq!(
        clip.name().map(|name| name.text()).as_deref(),
        Some("Caf\u{e9}")
    );
    assert!(!clip.has_as());
    let block = clip.body().and_then(|body| body.block()).expect("a block");
    let Some(BodyItem::Placement(note)) = block.items().next() else {
        panic!("a nested placement");
    };
    let Some(Expr::Literal(literal)) = note.default_arg().and_then(|arg| arg.value()) else {
        panic!("a literal default");
    };
    assert_eq!(
        literal
            .token()
            .map(|token| token.text().to_string())
            .as_deref(),
        Some("60")
    );
    assert!(note.name().is_none());
}
