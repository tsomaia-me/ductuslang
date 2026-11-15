// Review probe: for every node of each case, casts it to its typed view under
// `syntax::ast` and prints what each accessor returns, as source text.
use std::env;
use std::fs;

use symphonyscript::Source;
use symphonyscript::syntax::ast::*;
use symphonyscript::syntax::{NodeView, SyntaxKind, TokenView, parse};

fn t(node: Option<impl AstNodeText>) -> String {
    node.map_or("-".to_string(), |n| format!("`{}`", n.text_of()))
}

trait AstNodeText {
    fn text_of(&self) -> String;
}
impl<'p, N: AstNode<'p>> AstNodeText for N {
    fn text_of(&self) -> String {
        self.syntax().text().trim().replace('\n', "\\n")
    }
}
fn n(name: Option<Name<'_>>) -> String {
    name.map_or("-".to_string(), |n| format!("`{}`", n.text()))
}
fn k(token: Option<TokenView<'_>>) -> String {
    token.map_or("-".to_string(), |t| format!("`{}`", t.text()))
}
fn list<T: AstNodeText>(items: impl Iterator<Item = T>) -> String {
    let v: Vec<String> = items.map(|i| format!("`{}`", i.text_of())).collect();
    format!("[{}]", v.join(", "))
}
fn toks<'p>(items: impl Iterator<Item = TokenView<'p>>) -> String {
    let v: Vec<String> = items.map(|i| format!("`{}`", i.text())).collect();
    format!("[{}]", v.join(", "))
}

fn accessors(node: NodeView<'_>) -> Option<String> {
    use SyntaxKind as K;
    Some(match node.kind() {
        K::CellDecl => { let c = CellDecl::cast(node)?; format!("word={} has_capacity={} capacity={} name={} init={} ty={} clock={} has_rule={} value={} match_block={}", k(c.word()), c.has_capacity(), k(c.capacity()), n(c.name()), t(c.init()), t(c.ty()), t(c.clock()), c.has_rule(), t(c.value()), t(c.match_block())) }
        K::Init => { let c = Init::cast(node)?; format!("value={}", t(c.value())) }
        K::Clock => { let c = Clock::cast(node)?; format!("is_list={} entries={}", c.is_list(), list(c.entries())) }
        K::LetDecl => { let c = LetDecl::cast(node)?; format!("name={} ty={} value={} lambda={} match_block={}", n(c.name()), t(c.ty()), t(c.value()), t(c.lambda()), t(c.match_block())) }
        K::LocalLet => { let c = LocalLet::cast(node)?; format!("name={} ty={} value={} lambda={} match_block={}", n(c.name()), t(c.ty()), t(c.value()), t(c.lambda()), t(c.match_block())) }
        K::FnDecl => { let c = FnDecl::cast(node)?; format!("name={} generic_params={} params={} result={} body={}", n(c.name()), t(c.generic_params()), list(c.params()), t(c.result()), t(c.body())) }
        K::FnBody => { let c = FnBody::cast(node)?; format!("lets={} value={} match_block={}", list(c.lets()), t(c.value()), t(c.match_block())) }
        K::Param => { let c = Param::cast(node)?; format!("marker={} name={} ty={} default_value={}", k(c.marker()), n(c.name()), t(c.ty()), t(c.default_value())) }
        K::FnResult => { let c = FnResult::cast(node)?; format!("ty={}", t(c.ty())) }
        K::PatternDecl => { let c = PatternDecl::cast(node)?; format!("name={} generic_params={} params={} placement_type={} body={}", n(c.name()), t(c.generic_params()), list(c.params()), t(c.placement_type()), t(c.body())) }
        K::PatternParam => { let c = PatternParam::cast(node)?; format!("is_default={} is_children={} markers={} name={} ty={} placement_type={} default_value={}", c.is_default(), c.is_children(), toks(c.markers()), n(c.name()), t(c.ty()), t(c.placement_type()), t(c.default_value())) }
        K::TypeRef => { let c = TypeRef::cast(node)?; format!("base={} enum_name={} lengths={} capacity={}", k(c.base()), n(c.enum_name()), toks(c.lengths()), k(c.capacity())) }
        K::SumType => { let c = SumType::cast(node)?; format!("alternatives={}", list(c.alternatives())) }
        K::FnType => { let c = FnType::cast(node)?; format!("params={} result={}", list(c.params()), t(c.result())) }
        K::FnParam => { let c = FnParam::cast(node)?; format!("name={} ty={}", n(c.name()), t(c.ty())) }
        K::ParenType => { let c = ParenType::cast(node)?; format!("inner={}", t(c.inner())) }
        K::MapType => { let c = MapType::cast(node)?; format!("key_and_value={}", list(c.key_and_value())) }
        K::Size => { let c = Size::cast(node)?; format!("value={}", t(c.value())) }
        K::GenericParams => { let c = GenericParams::cast(node)?; format!("params={}", list(c.params())) }
        K::GenericParam => { let c = GenericParam::cast(node)?; format!("is_const={} name={} constraint={}", c.is_const(), n(c.name()), t(c.constraint())) }
        K::TypeDecl => { let c = TypeDecl::cast(node)?; format!("name={} generic_params={} alias={} fields={}", n(c.name()), t(c.generic_params()), t(c.alias()), list(c.fields())) }
        K::FieldDecl => { let c = FieldDecl::cast(node)?; format!("name={} ty={}", n(c.name()), t(c.ty())) }
        K::Placement => { let c = Placement::cast(node)?; format!("kind={} flags={} has_as={} name={} default_arg={} gate={} attrs={} body={}", n(c.kind()), t(c.flags()), c.has_as(), n(c.name()), t(c.default_arg()), t(c.gate()), t(c.attrs()), t(c.body())) }
        K::WirePlacement => { let c = WirePlacement::cast(node)?; format!("kind={} flags={} has_as={} name={} default_arg={} gate={} attrs={} destination={}", n(c.kind()), t(c.flags()), c.has_as(), n(c.name()), t(c.default_arg()), t(c.gate()), t(c.attrs()), t(c.destination())) }
        K::Flags => { let c = Flags::cast(node)?; format!("chars={:?}", c.chars()) }
        K::DefaultArg => { let c = DefaultArg::cast(node)?; format!("value={}", t(c.value())) }
        K::Gate => { let c = Gate::cast(node)?; format!("condition={}", t(c.condition())) }
        K::Attrs => { let c = Attrs::cast(node)?; format!("entries={}", list(c.entries())) }
        K::AttrEntry => { let c = AttrEntry::cast(node)?; format!("is_negated={} name={} value={}", c.is_negated(), n(c.name()), t(c.value())) }
        K::Body => { let c = Body::cast(node)?; format!("block={}", t(c.block())) }
        K::Block => { let c = Block::cast(node)?; format!("lets={} cells={} items={} members={}", list(c.lets()), list(c.cells()), list(c.items()), t(c.members())) }
        K::Line => { let c = Line::cast(node)?; format!("members={}", list(c.members())) }
        K::Members => { let c = Members::cast(node)?; format!("members={}", list(c.members())) }
        K::ParenPlacement => { let c = ParenPlacement::cast(node)?; format!("placement={}", t(c.placement())) }
        K::Bundle => { let c = Bundle::cast(node)?; format!("members={} bundle_for={}", list(c.members()), t(c.bundle_for())) }
        K::BundleFor => { let c = BundleFor::cast(node)?; let h = c.head(); format!("head.names={:?} head.iterable={} members={}", h.names().iter().map(|l| (l.name.text(), l.ty.as_ref().map(|t| t.text_of()))).collect::<Vec<_>>(), t(h.iterable()), list(c.members())) }
        K::ForBlock => { let c = ForBlock::cast(node)?; let h = c.head(); format!("head.names={:?} head.iterable={} block={}", h.names().iter().map(|l| (l.name.text(), l.ty.as_ref().map(|t| t.text_of()))).collect::<Vec<_>>(), t(h.iterable()), t(c.block())) }
        K::RangeExpr => { let c = RangeExpr::cast(node)?; format!("start={} end={}", t(c.start()), t(c.end())) }
        K::WhenBlock => { let c = WhenBlock::cast(node)?; format!("condition={} block={} guard_arms={} else_branch={}", t(c.condition()), t(c.block()), list(c.guard_arms()), t(c.else_branch())) }
        K::WhenElse => { let c = WhenElse::cast(node)?; format!("block={}", t(c.block())) }
        K::GuardArm => { let c = GuardArm::cast(node)?; format!("guard={} block={}", t(c.guard()), t(c.block())) }
        K::MatchBlock => { let c = MatchBlock::cast(node)?; format!("scrutinee={} arms={} else_arm={}", t(c.scrutinee()), list(c.arms()), t(c.else_arm())) }
        K::MatchArm => { let c = MatchArm::cast(node)?; format!("label={} alt_type={} value={}", t(c.label()), t(c.alt_type()), t(c.value())) }
        K::ElseArm => { let c = ElseArm::cast(node)?; format!("value={}", t(c.value())) }
        K::Destination => { let c = Destination::cast(node)?; format!("value={} match_block={}", t(c.value()), t(c.match_block())) }
        K::IfExpr => { let c = IfExpr::cast(node)?; format!("condition={} then_branch={} else_branch={}", t(c.condition()), t(c.then_branch()), t(c.else_branch())) }
        K::ForExpr | K::ForExprBlock => { let c = ForExpr::cast(node)?; format!("head={} is_block={} lets={} body={}", t(c.head()), c.is_block(), list(c.lets()), t(c.body())) }
        K::ForExprHead => { let c = ForExprHead::cast(node)?; let h = c.loop_head(); format!("loop_head.names={:?} loop_head.iterable={} accumulator={} accumulator_type={} start={}", h.names().iter().map(|l| (l.name.text(), l.ty.as_ref().map(|t| t.text_of()))).collect::<Vec<_>>(), t(h.iterable()), n(c.accumulator()), t(c.accumulator_type()), t(c.start())) }
        K::BinaryExpr => { let c = BinaryExpr::cast(node)?; format!("lhs={} op={} is_not={} rhs={} alt_type={}", t(c.lhs()), k(c.op()), c.is_not(), t(c.rhs()), t(c.alt_type())) }
        K::PrefixExpr => { let c = PrefixExpr::cast(node)?; format!("op={} operand={}", k(c.op()), t(c.operand())) }
        K::ParenExpr => { let c = ParenExpr::cast(node)?; format!("inner={}", t(c.inner())) }
        K::Literal => { let c = Literal::cast(node)?; format!("token={} kind={:?}", k(c.token()), c.token().map(|t| t.kind())) }
        K::StringLit => { let c = StringLit::cast(node)?; format!("parts={:?} is_terminated={}", c.parts().map(|p| match p { StringPart::Text(t) => format!("T:{}", t.text()), StringPart::Interpolation(i) => format!("I:{}", t(i.value())) }).collect::<Vec<_>>(), c.is_terminated()) }
        K::Interpolation => { let c = Interpolation::cast(node)?; format!("value={}", t(c.value())) }
        K::NameRef => { let c = NameRef::cast(node)?; format!("name={}", n(c.name())) }
        K::VariantRef => { let c = VariantRef::cast(node)?; format!("enum_name={} variant={}", n(c.enum_name()), n(c.variant())) }
        K::QualifiedName => { let c = QualifiedName::cast(node)?; format!("package={} name={}", n(c.package()), n(c.name())) }
        K::Conversion => { let c = Conversion::cast(node)?; format!("target={} operand={}", k(c.target()), t(c.operand())) }
        K::ArrayLit => { let c = ArrayLit::cast(node)?; format!("elements={}", list(c.elements())) }
        K::FieldExpr => { let c = FieldExpr::cast(node)?; format!("receiver={} field={}", t(c.receiver()), n(c.field())) }
        K::IndexExpr => { let c = IndexExpr::cast(node)?; format!("base={} is_slice={} index={} lower={} upper={}", t(c.base()), c.is_slice(), t(c.index()), t(c.lower()), t(c.upper())) }
        K::Bound => { let c = Bound::cast(node)?; format!("from_end={} value={}", c.from_end(), t(c.value())) }
        K::CallExpr => { let c = CallExpr::cast(node)?; format!("callee={} args={}", t(c.callee()), t(c.args())) }
        K::ArgList => { let c = ArgList::cast(node)?; format!("positional={} lambdas={} named={}", list(c.positional()), list(c.lambdas()), list(c.named())) }
        K::NamedArg => { let c = NamedArg::cast(node)?; format!("name={} value={}", n(c.name()), t(c.value())) }
        K::Lambda => { let c = Lambda::cast(node)?; format!("params={} body={}", t(c.params()), t(c.body())) }
        K::LambdaParams => { let c = LambdaParams::cast(node)?; format!("params={}", list(c.params())) }
        K::LambdaParam => { let c = LambdaParam::cast(node)?; format!("name={} ty={}", n(c.name()), t(c.ty())) }
        K::RecordLit => { let c = RecordLit::cast(node)?; format!("fields={}", list(c.fields())) }
        K::Field => { let c = Field::cast(node)?; format!("name={} value={}", n(c.name()), t(c.value())) }
        K::MapLit => { let c = MapLit::cast(node)?; format!("entries={}", list(c.entries())) }
        K::MapEntry => { let c = MapEntry::cast(node)?; format!("key_and_value={}", list(c.key_and_value())) }
        K::AltType => { let c = AltType::cast(node)?; format!("ty={}", t(c.ty())) }
        K::PlacementType => { let c = PlacementType::cast(node)?; format!("entries={}", list(c.entries())) }
        K::PlacementEntry => { let c = PlacementEntry::cast(node)?; format!("bundle={} count={}", t(c.bundle()), t(c.count())) }
        K::JsonLit => { let c = JsonLit::cast(node)?; format!("value={}", t(c.value())) }
        K::JsonObject => { let c = JsonObject::cast(node)?; format!("entries={}", list(c.entries())) }
        K::JsonEntry => { let c = JsonEntry::cast(node)?; format!("key={} value={}", k(c.key()), t(c.value())) }
        K::JsonArray => { let c = JsonArray::cast(node)?; format!("values={}", list(c.values())) }
        K::JsonAtom => { let c = JsonAtom::cast(node)?; format!("tokens={}", toks(c.tokens())) }
        K::PackageDecl => { let c = PackageDecl::cast(node)?; format!("record={}", t(c.record())) }
        K::Use => { let c = Use::cast(node)?; format!("names={}", list(c.names())) }
        K::Publish => { let c = Publish::cast(node)?; format!("names={}", list(c.names())) }
        K::File => { let c = File::cast(node)?; format!("items={}", c.items().count()) }
        _ => return None,
    })
}

fn walk(node: NodeView<'_>, depth: usize) {
    let text = node.text().trim().replace('\n', "\\n");
    let shown: String = text.chars().take(60).collect();
    match accessors(node) {
        Some(line) => println!("{}{:?} `{}` :: {}", "  ".repeat(depth + 1), node.kind(), shown, line),
        None => println!("{}{:?} `{}` (no accessors probed)", "  ".repeat(depth + 1), node.kind(), shown),
    }
    for child in node.children() {
        walk(child, depth + 1);
    }
}

fn main() {
    let path = env::args().nth(1).expect("a cases file");
    let all = fs::read_to_string(&path).expect("the cases file reads");
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let mut name = String::new();
    let mut text = String::new();
    let mut started = false;
    let mut run = |name: &str, text: &str| {
        let source = Source { path: "case.sym".into(), text: text.to_string() };
        let parsed = parse(&source, &vocabulary);
        println!("#### {name}  parse-diags={}", parsed.diagnostics.len());
        walk(parsed.root(), 0);
    };
    for line in all.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix("#### ") {
            if started { run(&name, &text); }
            started = true;
            name = rest.trim_end().to_string();
            text.clear();
        } else if started {
            text.push_str(line);
        }
    }
    if started { run(&name, &text); }
}
