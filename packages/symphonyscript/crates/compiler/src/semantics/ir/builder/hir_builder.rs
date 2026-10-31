use std::ops::Range;

use symphonyscript_core::Vocabulary;

use super::bodies::skipped_names;
use super::faults::Faults;
use super::index::index;
use super::scalar_type::scalar_type;
use crate::semantics::Internal;
use crate::semantics::ir::{
    Expr, ExprId, ExprKind, FnDecl, FnId, Hir, Ident, LetDecl, LetId, Loc, Param, PatternDecl,
    PatternId, PlacementId, Top, TypeBase, TypeRef, UnknownNames,
};
use crate::syntax::ast::{self, AstNode, Item, Name};
use crate::syntax::{ElementView, NodeView, SyntaxKind, Walk};

pub(crate) struct HirBuilder<'h> {
    pub(super) hir: &'h mut Hir,
    pub(super) file: u32,
    pub(super) faults: Faults,
    pub(super) vocabulary: &'h Vocabulary,
    pub(super) dropped: bool,
}

fn param_regions<'p>(decl: &NodeView<'p>, list: SyntaxKind) -> Vec<NodeView<'p>> {
    let inside = decl
        .children()
        .filter(|child| child.kind() == list)
        .flat_map(|list| list.children());
    decl.children()
        .chain(inside)
        .filter(|child| child.kind() == SyntaxKind::ErrorNode)
        .collect()
}

fn holds_default(region: &NodeView<'_>) -> bool {
    region.preorder().any(|event| {
        matches!(event, Walk::Enter(ElementView::Token(token)) if token.kind() == SyntaxKind::DefaultKw)
    })
}

fn skipped_params(regions: &[NodeView<'_>]) -> UnknownNames {
    UnknownNames::of(regions.iter().flat_map(skipped_names).collect())
}

impl HirBuilder<'_> {
    pub(super) fn item(&mut self, item: Item<'_>) -> Result<Option<PlacementId>, Internal> {
        match item {
            Item::Let(node) => {
                let loc = self.loc(node.syntax().range());
                let value = match node.match_block() {
                    Some(block) => self.match_block(&block),
                    None => self.expr(node.value(), loc),
                };
                let decl = LetDecl {
                    name: self.ident(node.name()),
                    ty: node.ty().map(|ty| self.type_ref(&ty)),
                    value,
                    loc,
                };
                let id = LetId(index(self.hir.lets.len()));
                self.hir.lets.push(decl);
                self.hir.top.push(Top::Let(id));
            }
            Item::Cell(node) => self.cell(&node),
            Item::Fn(node) if self.unbuilt_fn(&node) => self.placeholder(node.name()),
            Item::Fn(node) => {
                let loc = self.loc(node.syntax().range());
                let params = node
                    .params()
                    .map(|param| Param {
                        name: self.ident(param.name()),
                        default: param
                            .default_value()
                            .map(|value| self.expr(Some(value), loc)),
                        is_default: false,
                        loc: self.loc(param.syntax().range()),
                    })
                    .collect();
                let (lets, value) = match node.body() {
                    Some(body) => {
                        let lets = body.lets().map(|local| self.local_let(&local)).collect();
                        let value = match body.match_block() {
                            Some(block) => self.match_block(&block),
                            None => self.expr(body.value(), loc),
                        };
                        (lets, value)
                    }
                    None => (Vec::new(), self.push(ExprKind::Error, loc)),
                };
                let regions = param_regions(node.syntax(), SyntaxKind::ParamList);
                let decl = FnDecl {
                    name: self.ident(node.name()),
                    params,
                    skipped_params: skipped_params(&regions),
                    faulty_head: !regions.is_empty(),
                    lets,
                    value,
                    loc,
                };
                let id = FnId(index(self.hir.fns.len()));
                self.hir.fns.push(decl);
                self.hir.top.push(Top::Fn(id));
            }
            Item::Pattern(node) if self.unbuilt_pattern(&node) => self.placeholder(node.name()),
            Item::Pattern(node) => {
                let loc = self.loc(node.syntax().range());
                let regions = param_regions(node.syntax(), SyntaxKind::PatternParamList);
                let params = node
                    .params()
                    .map(|param| Param {
                        name: self.ident(param.name()),
                        default: param
                            .default_value()
                            .map(|value| self.expr(Some(value), loc)),
                        is_default: param.is_default(),
                        loc: self.loc(param.syntax().range()),
                    })
                    .collect();
                let (lets, body) = match node.body() {
                    Some(block) => {
                        let lets = block.lets().map(|local| self.local_let(&local)).collect();
                        (lets, self.block(block)?)
                    }
                    None => (Vec::new(), self.absent_block()),
                };
                let decl = PatternDecl {
                    name: self.ident(node.name()),
                    params,
                    skipped_params: skipped_params(&regions),
                    skipped_default: regions.iter().any(holds_default),
                    lets,
                    body,
                    loc,
                };
                let id = PatternId(index(self.hir.patterns.len()));
                self.hir.patterns.push(decl);
                self.hir.top.push(Top::Pattern(id));
            }
            Item::Placement(node) => {
                let id = self.top_placement(&node)?;
                self.hir.top.push(Top::Placement(id));
                return Ok(Some(id));
            }
            Item::Wire(_) => {}
            Item::Type(node) => {
                if node.alias().is_some() || node.fields().next().is_some() {
                    self.draw_item(node.syntax(), "a `type` declaration (SPEC §3.8)");
                }
                self.placeholder(node.name());
            }
            Item::Package(node) => self.draw_item(
                node.syntax(),
                "the `Package` item of a package file (SPEC §15.2)",
            ),
            Item::Require(node) => {
                self.draw_item(node.syntax(), "a `require` item (SPEC §15.3)");
            }
            Item::Use(node) => self.draw_item(node.syntax(), "a `use` item (SPEC §15.4)"),
            Item::Publish(node) => {
                self.draw_item(node.syntax(), "a `publish` item (SPEC §15.5)");
            }
        }
        Ok(None)
    }

    pub(super) fn local_let(&mut self, node: &ast::LocalLet<'_>) -> LetDecl {
        let loc = self.loc(node.syntax().range());
        let value = match node.match_block() {
            Some(block) => self.match_block(&block),
            None => self.expr(node.value(), loc),
        };
        LetDecl {
            name: self.ident(node.name()),
            ty: None,
            value,
            loc,
        }
    }

    pub(super) fn type_ref(&mut self, ty: &ast::Type<'_>) -> TypeRef {
        if missing_type(ty.syntax()) {
            return TypeRef {
                base: TypeBase::Error,
                lengths: Vec::new(),
                capacity: None,
                loc: self.loc(ty.syntax().range()),
            };
        }
        let node = match ty {
            ast::Type::Ref(node) => node,
            ast::Type::Sum(node) => {
                return self.unbuilt_type(node.syntax().range(), "a sum type (SPEC §3.9)");
            }
            ast::Type::Fn(node) => {
                return self.unbuilt_type(node.syntax().range(), "a function type (SPEC §6.7)");
            }
        };
        let loc = self.loc(node.syntax().range());
        if !self.faults.touch(node.syntax().range())
            && !unclosed(node)
            && let Some((range, construct)) = unbuilt_part(node)
        {
            return self.unbuilt_type(range, construct);
        }
        let base = match node.base() {
            _ if self.faults.touch(node.syntax().range()) || unclosed(node) => TypeBase::Error,
            Some(token) => match scalar_type(&token) {
                Some(scalar) => TypeBase::Scalar(scalar),
                None => match self.ident(node.enum_name()) {
                    Some(name) => TypeBase::Enum(name),
                    None => TypeBase::Error,
                },
            },
            None => TypeBase::Error,
        };
        let lengths = node
            .lengths()
            .map(|token| (length_value(token.text()), self.loc(token.range())))
            .collect();
        let capacity = node
            .capacity()
            .map(|token| (length_value(token.text()), self.loc(token.range())));
        TypeRef {
            base,
            lengths,
            capacity,
            loc,
        }
    }

    fn unbuilt_type(&mut self, range: Range<u32>, construct: &str) -> TypeRef {
        let loc = self.loc(range.clone());
        self.draw_unbuilt(range, construct);
        TypeRef {
            base: TypeBase::Error,
            lengths: Vec::new(),
            capacity: None,
            loc,
        }
    }

    pub(super) fn header_names(&self, item: &NodeView<'_>) -> Vec<String> {
        let mut names = Vec::new();
        let Some(fault) = self.faults.first_in(item.range()) else {
            return names;
        };
        let mut token = item.first_token();
        while let Some(current) = token {
            crate::semantics::counted_walk::CountedWalk::HirBuilderHeaderNames.step();
            if matches!(current.kind(), SyntaxKind::Newline | SyntaxKind::Indent)
                || current.range().start >= item.range().end
            {
                break;
            }
            if current.kind() == SyntaxKind::Ident && current.range().start >= fault {
                names.push(Name::new(current.clone()).text());
            }
            token = current.next_token();
        }
        names
    }

    pub(super) fn ident(&self, name: Option<Name<'_>>) -> Option<Ident> {
        let name = name?;
        if self.faults.touch(name.range()) {
            return None;
        }
        Some(Ident {
            text: name.text(),
            loc: self.loc(name.range()),
        })
    }

    pub(super) fn loc(&self, range: Range<u32>) -> Loc {
        Loc::new(self.file, range)
    }

    pub(super) fn push(&mut self, kind: ExprKind, loc: Loc) -> ExprId {
        let id = ExprId(index(self.hir.exprs.len()));
        self.hir.exprs.push(Expr { kind, loc });
        id
    }
}

pub(super) fn digits_value(text: &str) -> Option<u64> {
    let digits: String = text.chars().filter(|c| *c != '_').collect();
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(digits.parse::<u64>().unwrap_or(u64::MAX))
}

fn length_value(text: &str) -> Option<u32> {
    digits_value(text).map(|value| u32::try_from(value).unwrap_or(u32::MAX))
}

fn unclosed(node: &ast::TypeRef<'_>) -> bool {
    let depth = node
        .syntax()
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .fold(0i32, |depth, token| match token.kind() {
            SyntaxKind::LBracket => depth + 1,
            SyntaxKind::RBracket => depth - 1,
            _ => depth,
        });
    depth != 0
}

fn missing_type(node: &NodeView<'_>) -> bool {
    node.preorder().any(|event| {
        matches!(
            event,
            Walk::Enter(ElementView::Node(child))
                if child.kind() == SyntaxKind::TypeRef && child.range().is_empty()
        )
    })
}

fn unbuilt_part(node: &ast::TypeRef<'_>) -> Option<(Range<u32>, &'static str)> {
    let whole = node.syntax().range();
    let mut brackets = 0usize;
    let mut capacity_at = None;
    for element in node.syntax().children_with_tokens() {
        let found = match &element {
            ElementView::Node(child) => match child.kind() {
                SyntaxKind::StrType => Some("the string type `str` (SPEC §3.3)"),
                SyntaxKind::MapType => Some("a map type (SPEC §3.10)"),
                SyntaxKind::JsonType => Some("the document type `json` (SPEC §3.11)"),
                SyntaxKind::ParenType => Some("a parenthesized type (SPEC §3.2)"),
                SyntaxKind::QualifiedName => Some("a qualified name (SPEC §15.3)"),
                SyntaxKind::TypeArgs => Some("a list of bracket arguments of a type (SPEC §6.8)"),
                SyntaxKind::Size => Some("a size written as an expression (SPEC §3.2)"),
                _ => None,
            },
            ElementView::Token(token) => match token.kind() {
                SyntaxKind::NoneKw => Some("the type `none` (SPEC §3.9)"),
                SyntaxKind::LBracket => {
                    brackets += 1;
                    None
                }
                SyntaxKind::Dot2 => {
                    capacity_at = capacity_at.or(Some(brackets));
                    None
                }
                _ => None,
            },
        };
        if let Some(construct) = found {
            return Some((element.range(), construct));
        }
    }
    if capacity_at.is_some_and(|at| at < brackets) {
        return Some((
            whole,
            "a capacity at an inner level of an array type (SPEC §3.2)",
        ));
    }
    if node
        .syntax()
        .children_with_tokens()
        .any(|element| element.kind() == SyntaxKind::Question)
    {
        return Some((whole, "an optional type `T?` (SPEC §3.9)"));
    }
    None
}
