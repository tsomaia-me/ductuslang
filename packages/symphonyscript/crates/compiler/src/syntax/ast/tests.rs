use rowan::{GreenNodeBuilder, Language};

use super::{AstNode, BodyItem, Expr, Item};
use crate::syntax::{SymphonyLanguage, SyntaxKind, SyntaxNode};

fn tree(shape: &[(Option<SyntaxKind>, SyntaxKind, &str)]) -> SyntaxNode {
    let mut builder = GreenNodeBuilder::new();
    for (open, kind, text) in shape {
        match open {
            Some(SyntaxKind::Dedent) => builder.finish_node(),
            Some(_) => builder.start_node(SymphonyLanguage::kind_to_raw(*kind)),
            None => builder.token(SymphonyLanguage::kind_to_raw(*kind), text),
        }
    }
    SyntaxNode::new_root(builder.finish())
}

#[test]
fn the_typed_view_reads_a_hand_built_tree() {
    use SyntaxKind::*;
    let start = |kind| (Some(Indent), kind, "");
    let end = (Some(Dedent), Error, "");
    let token = |kind, text| (None, kind, text);
    let root = tree(&[
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
    assert_eq!(root.to_string(), "Clip Cafe\u{301}:\n    Note/60\n");
    let file = super::File::cast(root).expect("a file");
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
