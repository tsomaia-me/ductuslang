mod support;

use symphonyscript::syntax::ast::{
    AstNode, BodyItem, Expr, Item, Iterable, Member, Name, StringPart,
};
use symphonyscript::syntax::{Parsed, SyntaxKind};

fn clean(text: &str) -> Parsed {
    let parsed = support::parsed(text);
    assert_eq!(parsed.diagnostics, vec![], "{text:?}");
    parsed
}

fn items<'p>(parsed: &'p Parsed) -> Vec<Item<'p>> {
    parsed.file().expect("the root is a file").items().collect()
}

fn text(name: Option<Name<'_>>) -> String {
    name.map(|name| name.text()).unwrap_or_default()
}

fn source(expr: Option<Expr<'_>>) -> String {
    expr.map(|expr| expr.syntax().to_string().trim().to_string())
        .unwrap_or_default()
}

#[test]
fn declarations_expose_their_parts() {
    let parsed = clean(
        "input fine: f32[2] = 0\n\
         let level = match wave:\n    Sine: 1.0\n    -1: 0.5\n    else: 0.6\n\
         fn scale(degree, root = 60):\n    let octave = degree div 7\n    root + octave\n\
         pattern Arp(default root, steps = 4):\n    let k = steps\n    Note/root\n",
    );
    let items = items(&parsed);
    let Item::Input(input) = &items[0] else {
        panic!("an input");
    };
    assert_eq!(text(input.name()), "fine");
    let ty = input.ty().expect("a type");
    assert_eq!(ty.base().map(|token| token.kind()), Some(SyntaxKind::F32Kw));
    assert!(ty.enum_name().is_none());
    assert_eq!(
        ty.lengths()
            .map(|token| token.text().to_string())
            .collect::<Vec<_>>(),
        ["2"]
    );
    assert_eq!(source(input.value()), "0");

    let Item::Let(level) = &items[1] else {
        panic!("a let");
    };
    assert!(level.value().is_none());
    let block = level.match_block().expect("a match");
    assert_eq!(source(block.scrutinee()), "wave");
    let arms: Vec<_> = block.arms().collect();
    assert_eq!(source(arms[0].label()), "Sine");
    assert_eq!(source(arms[1].label()), "-1");
    assert_eq!(source(arms[1].value()), "0.5");
    assert_eq!(source(block.else_arm().and_then(|arm| arm.value())), "0.6");

    let Item::Fn(scale) = &items[2] else {
        panic!("a function");
    };
    assert_eq!(text(scale.name()), "scale");
    let params: Vec<_> = scale.params().collect();
    assert_eq!(params.len(), 2);
    assert_eq!(text(params[1].name()), "root");
    assert_eq!(source(params[1].default_value()), "60");
    assert!(params[0].default_value().is_none());
    let body = scale.body().expect("a body");
    let lets: Vec<_> = body.lets().collect();
    assert_eq!(text(lets[0].name()), "octave");
    assert_eq!(source(lets[0].value()), "degree div 7");
    assert_eq!(source(body.value()), "root + octave");
    assert!(body.match_block().is_none());

    let Item::Pattern(arp) = &items[3] else {
        panic!("a pattern");
    };
    let params: Vec<_> = arp.params().collect();
    assert!(params[0].is_default());
    assert!(!params[1].is_default());
    assert_eq!(source(params[1].default_value()), "4");
    let body = arp.body().expect("a body");
    assert_eq!(body.lets().count(), 1);
    assert_eq!(body.items().count(), 1);
}

#[test]
fn placements_expose_their_parts() {
    let parsed = clean(
        "Clip as verse | gain=0.8:\n\
         \x20   Note'* as hook /65 when on | velocity=0.9 accent !accent\n\
         \x20   Plays as back /0.5 | transpose=-2: [verse, verse][0]\n\
         \x20   Note Note/1 (Note when on) [Note for i in 0..2: Note/i]\n\
         \x20   for s in list: Note\n\
         \x20   when on:\n        Note\n    else: Note\n\
         \x20   when:\n        a: Note\n        else: Note\n",
    );
    let items = items(&parsed);
    let Item::Placement(verse) = &items[0] else {
        panic!("a placement");
    };
    assert_eq!(text(verse.kind()), "Clip");
    assert_eq!(text(verse.name()), "verse");
    assert!(verse.has_as());
    let entries: Vec<_> = verse.attrs().expect("attrs").entries().collect();
    assert_eq!(text(entries[0].name()), "gain");
    let block = verse.body().and_then(|body| body.block()).expect("a block");
    let body: Vec<BodyItem> = block.items().collect();
    assert_eq!(body.len(), 6);

    let BodyItem::Placement(hook) = &body[0] else {
        panic!("a placement");
    };
    assert_eq!(hook.flags().expect("flags").chars(), ['\'', '*']);
    assert_eq!(text(hook.name()), "hook");
    assert_eq!(source(hook.default_arg().and_then(|arg| arg.value())), "65");
    assert_eq!(source(hook.gate().and_then(|gate| gate.condition())), "on");
    let entries: Vec<_> = hook.attrs().expect("attrs").entries().collect();
    assert_eq!(entries.len(), 3);
    assert_eq!(source(entries[0].value()), "0.9");
    assert!(!entries[1].is_negated() && entries[1].value().is_none());
    assert!(entries[2].is_negated());
    assert_eq!(text(entries[2].name()), "accent");
    assert!(hook.body().is_none());

    let BodyItem::Wire(back) = &body[1] else {
        panic!("a wire");
    };
    assert_eq!(text(back.kind()), "Plays");
    assert_eq!(text(back.name()), "back");
    assert_eq!(
        source(back.default_arg().and_then(|arg| arg.value())),
        "0.5"
    );
    let destination = back.destination().expect("a destination");
    assert_eq!(source(destination.value()), "[verse, verse][0]");
    assert!(destination.match_block().is_none());

    let BodyItem::Line(line) = &body[2] else {
        panic!("a line");
    };
    let members: Vec<Member> = line.members().collect();
    assert_eq!(members.len(), 4);
    let Member::Paren(paren) = &members[2] else {
        panic!("a parenthesized placement");
    };
    let inner = paren.placement().expect("a placement");
    assert!(inner.gate().is_some());
    let Member::Bundle(bundle) = &members[3] else {
        panic!("a bundle");
    };
    assert_eq!(bundle.members().count(), 1);
    let bundle_for = bundle.bundle_for().expect("a bundle for");
    assert_eq!(text(bundle_for.variable()), "i");
    assert!(matches!(bundle_for.iterable(), Some(Iterable::Range(_))));
    assert_eq!(bundle_for.members().count(), 1);

    let BodyItem::For(each) = &body[3] else {
        panic!("a for");
    };
    assert_eq!(text(each.variable()), "s");
    let Some(Iterable::Expr(list)) = each.iterable() else {
        panic!("an array iterable");
    };
    assert_eq!(list.syntax().to_string().trim(), "list");
    let inline = each
        .block()
        .and_then(|block| block.members())
        .expect("inline members");
    assert_eq!(inline.members().count(), 1);

    let BodyItem::When(gated) = &body[4] else {
        panic!("a when block");
    };
    assert_eq!(source(gated.condition()), "on");
    assert_eq!(gated.block().map(|block| block.items().count()), Some(1));
    assert!(
        gated
            .else_branch()
            .and_then(|branch| branch.block())
            .is_some()
    );

    let BodyItem::When(guarded) = &body[5] else {
        panic!("a guarded when block");
    };
    assert!(guarded.condition().is_none());
    let arms: Vec<_> = guarded.guard_arms().collect();
    assert_eq!(source(arms[0].guard()), "a");
    assert!(arms[0].block().is_some());
    assert!(guarded.else_branch().is_some());
}

#[test]
fn expressions_expose_their_parts() {
    let parsed = clean(
        "let a = if c then x is not y else -z\n\
         let b = f(1, 2)[0].gain + g(n=1)\n\
         let c = [\"kit/{i}.wav\", Wave::Saw, i32(q), (p)]\n",
    );
    let items = items(&parsed);
    let value = |index: usize| match &items[index] {
        Item::Let(decl) => decl.value().expect("a value"),
        _ => panic!("a let"),
    };

    let Expr::If(choice) = value(0) else {
        panic!("an if");
    };
    assert_eq!(source(choice.condition()), "c");
    let Some(Expr::Binary(compare)) = choice.then_branch() else {
        panic!("a comparison");
    };
    assert!(compare.is_not());
    assert_eq!(compare.op().map(|op| op.kind()), Some(SyntaxKind::IsKw));
    assert_eq!(source(compare.lhs()), "x");
    assert_eq!(source(compare.rhs()), "y");
    let Some(Expr::Prefix(negate)) = choice.else_branch() else {
        panic!("a negation");
    };
    assert_eq!(negate.op().map(|op| op.kind()), Some(SyntaxKind::Minus));
    assert_eq!(source(negate.operand()), "z");

    let Expr::Binary(sum) = value(1) else {
        panic!("a sum");
    };
    assert!(!sum.is_not());
    let Some(Expr::Field(field)) = sum.lhs() else {
        panic!("a field read");
    };
    assert_eq!(text(field.field()), "gain");
    let Some(Expr::Index(index)) = field.receiver() else {
        panic!("an index");
    };
    assert_eq!(source(index.index().and_then(|bound| bound.value())), "0");
    let Some(Expr::Call(call)) = index.base() else {
        panic!("a call");
    };
    assert_eq!(source(call.callee()), "f");
    let args = call.args().expect("arguments");
    assert_eq!(args.positional().count(), 2);
    assert_eq!(args.named().count(), 0);
    let Some(Expr::Call(named)) = sum.rhs() else {
        panic!("a named call");
    };
    let named: Vec<_> = named.args().expect("arguments").named().collect();
    assert_eq!(text(named[0].name()), "n");
    assert_eq!(source(named[0].value()), "1");

    let Expr::Array(array) = value(2) else {
        panic!("an array");
    };
    let elements: Vec<Expr> = array.elements().collect();
    let Expr::String(string) = &elements[0] else {
        panic!("a string");
    };
    assert!(string.is_terminated());
    let parts: Vec<StringPart> = string.parts().collect();
    assert_eq!(parts.len(), 3);
    let StringPart::Interpolation(interpolation) = &parts[1] else {
        panic!("an interpolation");
    };
    assert_eq!(source(interpolation.value()), "i");
    let Expr::VariantRef(variant) = &elements[1] else {
        panic!("a variant");
    };
    assert_eq!(text(variant.enum_name()), "Wave");
    assert_eq!(text(variant.variant()), "Saw");
    let Expr::Conversion(conversion) = &elements[2] else {
        panic!("a conversion");
    };
    assert_eq!(
        conversion.target().map(|token| token.kind()),
        Some(SyntaxKind::I32Kw)
    );
    assert_eq!(source(conversion.operand()), "q");
    let Expr::Paren(paren) = &elements[3] else {
        panic!("a parenthesized expression");
    };
    assert_eq!(source(paren.inner()), "p");
}

#[test]
fn names_compare_in_nfc() {
    let parsed = clean("Clip Cafe\u{301}\nClip Caf\u{e9}x\n");
    let items = items(&parsed);
    let Item::Placement(first) = &items[0] else {
        panic!("a placement");
    };
    assert_eq!(text(first.name()), "Caf\u{e9}");
    assert_eq!(first.name().expect("a name").token().text(), "Cafe\u{301}");
    let Item::Placement(second) = &items[1] else {
        panic!("a placement");
    };
    assert_eq!(text(second.name()), "Caf\u{e9}x");
}

#[test]
fn literals_and_to_are_tokens_of_their_nodes() {
    let parsed = clean("Clip c:\n    Plays | gain=to.gain: c\nlet t = true\n");
    let items = items(&parsed);
    let Item::Let(decl) = &items[1] else {
        panic!("a let");
    };
    let Some(Expr::Literal(literal)) = decl.value() else {
        panic!("a literal");
    };
    assert_eq!(
        literal.token().map(|token| token.kind()),
        Some(SyntaxKind::TrueKw)
    );
    let Item::Placement(clip) = &items[0] else {
        panic!("a placement");
    };
    let block = clip.body().and_then(|body| body.block()).expect("a block");
    let Some(BodyItem::Wire(wire)) = block.items().next() else {
        panic!("a wire");
    };
    let entry = wire
        .attrs()
        .and_then(|attrs| attrs.entries().next())
        .expect("an entry");
    let Some(Expr::Field(field)) = entry.value() else {
        panic!("a field read");
    };
    assert!(matches!(field.receiver(), Some(Expr::To(_))));
}

fn parse_broken(text: &str) -> Parsed {
    let parsed = support::parsed(text);
    assert!(!parsed.diagnostics.is_empty(), "{text:?}");
    parsed
}

fn let_value(parsed: &Parsed, index: usize) -> Expr<'_> {
    match items(parsed).into_iter().nth(index) {
        Some(Item::Let(decl)) => decl.value().expect("a value"),
        other => panic!("a let, found {other:?}"),
    }
}

#[test]
fn a_missing_operand_leaves_the_others_in_place() {
    let broken = parse_broken("let a = if then 1 else 2\nlet b = 1 +\nlet c = * 2\nlet d = a[]\n");
    let Expr::If(choice) = let_value(&broken, 0) else {
        panic!("an if");
    };
    assert!(choice.condition().is_none());
    assert_eq!(source(choice.then_branch()), "1");
    assert_eq!(source(choice.else_branch()), "2");
    let Expr::Binary(sum) = let_value(&broken, 1) else {
        panic!("a sum");
    };
    assert_eq!(source(sum.lhs()), "1");
    assert!(sum.rhs().is_none());
    let Expr::Binary(product) = let_value(&broken, 2) else {
        panic!("a product");
    };
    assert!(product.lhs().is_none());
    assert_eq!(source(product.rhs()), "2");
    let Expr::Index(index) = let_value(&broken, 3) else {
        panic!("an index");
    };
    assert_eq!(source(index.base()), "a");
    assert!(index.index().is_none());

    let broken = parse_broken("let e = if c then else 3\n");
    let Expr::If(choice) = let_value(&broken, 0) else {
        panic!("an if");
    };
    assert_eq!(source(choice.condition()), "c");
    assert!(choice.then_branch().is_none());
    assert_eq!(source(choice.else_branch()), "3");
}

#[test]
fn a_missing_range_bound_or_arm_label_leaves_the_other_in_place() {
    let broken = parse_broken("Clip c:\n    for i in ..3:\n        Note\n");
    let Some(Item::Placement(clip)) = items(&broken).into_iter().next() else {
        panic!("a placement");
    };
    let block = clip.body().and_then(|body| body.block()).expect("a block");
    let Some(BodyItem::For(each)) = block.items().next() else {
        panic!("a for");
    };
    let Some(Iterable::Range(range)) = each.iterable() else {
        panic!("a range");
    };
    assert!(range.start().is_none());
    assert_eq!(source(range.end()), "3");

    let broken = parse_broken("let a = match b:\n    : 1\n    else: 2\n");
    let Some(Item::Let(decl)) = items(&broken).into_iter().next() else {
        panic!("a let");
    };
    let arm = decl
        .match_block()
        .and_then(|block| block.arms().next())
        .expect("an arm");
    assert!(arm.label().is_none());
    assert_eq!(source(arm.value()), "1");
}
