use super::hir_builder::HirBuilder;
use super::index::index;
use crate::semantics::Internal;
use crate::semantics::ir::{
    Attr, AttrValue, Block, BlockId, BodyItem, BundleFor, ExprKind, Iterable, Loc, LoopVar,
    Placement, PlacementId, UnknownNames, WhenArm, Wire, WireId,
};
use crate::syntax::ast::{self, AstNode, BodyItem as AstBodyItem, Member, Name};
use crate::syntax::{ElementView, NodeView, SyntaxKind, Walk};

fn significant(kind: SyntaxKind) -> bool {
    !kind.is_trivia()
        && !matches!(
            kind,
            SyntaxKind::Newline | SyntaxKind::Indent | SyntaxKind::Dedent
        )
}

pub(super) fn skipped_names(node: &NodeView<'_>) -> Vec<String> {
    node.preorder()
        .filter_map(|event| match event {
            Walk::Enter(ElementView::Token(token)) if token.kind() == SyntaxKind::Ident => {
                Some(Name::new(token).text())
            }
            Walk::Enter(_) | Walk::Leave(_) => None,
        })
        .collect()
}

pub(super) fn error_names(node: &NodeView<'_>) -> Vec<String> {
    let mut names = Vec::new();
    let mut stack: Vec<NodeView<'_>> = node.children().collect();
    while let Some(current) = stack.pop() {
        crate::semantics::counted_walk::CountedWalk::BodiesErrorNames.step();
        match current.kind() {
            SyntaxKind::ErrorNode => names.extend(skipped_names(&current)),
            SyntaxKind::Body => {}
            _ => stack.extend(current.children()),
        }
    }
    names
}

pub(super) fn skipped_block(node: &NodeView<'_>) -> bool {
    node.kind() == SyntaxKind::ErrorNode
        && node
            .children_with_tokens()
            .find_map(|element| match element {
                ElementView::Token(token) if token.kind().is_trivia() => None,
                ElementView::Token(token) => Some(token.kind() == SyntaxKind::Indent),
                ElementView::Node(_) => Some(false),
            })
            .unwrap_or(false)
}

fn head(node: &NodeView<'_>) -> std::ops::Range<u32> {
    let range = node.range();
    let mut end = range.start;
    for element in node.children_with_tokens() {
        match element {
            ElementView::Node(child) if child.kind() == SyntaxKind::Body => break,
            ElementView::Node(child) => {
                let mut token = child.last_token();
                while let Some(current) = token.clone() {
                    crate::semantics::counted_walk::CountedWalk::BodiesHead.step();
                    if current.range().start < child.range().start {
                        token = None;
                        break;
                    }
                    if significant(current.kind()) {
                        break;
                    }
                    token = current.prev_token();
                }
                if let Some(token) = token {
                    end = end.max(token.range().end);
                }
            }
            ElementView::Token(token) => {
                if token.kind() == SyntaxKind::Newline {
                    break;
                }
                if significant(token.kind()) {
                    end = end.max(token.range().end);
                }
            }
        }
    }
    range.start..end.max(range.start)
}

impl<'p> HirBuilder<'_> {
    pub(super) fn top_placement(
        &mut self,
        node: &ast::Placement<'p>,
    ) -> Result<PlacementId, Internal> {
        let mut pending = Vec::new();
        let id = self.placement(node, false, &mut pending);
        self.fill(pending)?;
        Ok(id)
    }

    pub(super) fn block(&mut self, node: ast::Block<'p>) -> Result<BlockId, Internal> {
        let id = self.empty_block();
        self.fill(vec![(node, id)])?;
        Ok(id)
    }

    pub(super) fn absent_block(&mut self) -> BlockId {
        let id = BlockId(index(self.hir.blocks.len()));
        self.hir.blocks.push(Block {
            items: Vec::new(),
            unknown: UnknownNames {
                every: true,
                names: Vec::new(),
            },
        });
        id
    }

    pub(super) fn empty_block(&mut self) -> BlockId {
        let id = BlockId(index(self.hir.blocks.len()));
        self.hir.blocks.push(Block::default());
        id
    }

    pub(super) fn fill(
        &mut self,
        mut pending: Vec<(ast::Block<'p>, BlockId)>,
    ) -> Result<(), Internal> {
        while let Some((node, id)) = pending.pop() {
            crate::semantics::counted_walk::CountedWalk::BodiesFill.step();
            let mut items = Vec::new();
            let mut skipped = Vec::new();
            let mut previous = None;
            for child in node.syntax().children() {
                if child.kind() == SyntaxKind::ErrorNode {
                    match previous.take() {
                        Some(placement) if skipped_block(&child) => {
                            self.skip_block_of(placement, skipped_names(&child))?;
                        }
                        _ => skipped.extend(skipped_names(&child)),
                    }
                } else if let Some(item) = AstBodyItem::cast(child.clone()) {
                    let lone = matches!(item, AstBodyItem::Placement(_));
                    let before = items.len();
                    self.dropped = false;
                    self.body_item(item, &mut items, &mut pending);
                    if self.dropped {
                        skipped.extend(skipped_names(&child));
                    } else {
                        skipped.extend(error_names(&child));
                    }
                    previous = match items.get(before..) {
                        Some([BodyItem::Placement(placement)]) if lone => Some(*placement),
                        _ => None,
                    };
                }
            }
            if let Some(members) = node.members() {
                for (position, member) in members.members().enumerate() {
                    let syntax = member.syntax().clone();
                    self.dropped = false;
                    self.member(member, position > 0, &mut items, &mut pending);
                    if self.dropped {
                        skipped.extend(skipped_names(&syntax));
                    } else {
                        skipped.extend(error_names(&syntax));
                    }
                }
            }
            let mut unknown = UnknownNames::of(skipped);
            unknown.every = items.is_empty();
            let block = self
                .hir
                .blocks
                .get_mut(id.index())
                .ok_or(Internal::missed("block"))?;
            block.items = items;
            block.unknown = unknown;
        }
        Ok(())
    }

    pub(super) fn skip_block_of(
        &mut self,
        placement: PlacementId,
        names: Vec<String>,
    ) -> Result<(), Internal> {
        self.hir
            .placements
            .get_mut(placement.index())
            .ok_or(Internal::missed("placement"))?
            .skipped_block = UnknownNames::of(names);
        Ok(())
    }

    fn body_item(
        &mut self,
        item: AstBodyItem<'p>,
        items: &mut Vec<BodyItem>,
        pending: &mut Vec<(ast::Block<'p>, BlockId)>,
    ) {
        match item {
            AstBodyItem::Placement(node) => {
                if let Some(id) = self.node_placement(&node, false, pending) {
                    items.push(BodyItem::Placement(id));
                }
            }
            AstBodyItem::Wire(node) => match self.wire(&node) {
                Some(id) => items.push(BodyItem::Wire(id)),
                None => self.dropped = true,
            },
            AstBodyItem::Line(node) => {
                for (position, member) in node.members().enumerate() {
                    self.member(member, position > 0, items, pending);
                }
            }
            AstBodyItem::For(node) => {
                let loc = self.loc(node.syntax().range());
                let (names, iterable) = self.loop_head(&node.head(), loc);
                let body = self.nested(node.block(), pending);
                items.push(BodyItem::For {
                    names,
                    iterable,
                    body,
                    loc,
                });
            }
            AstBodyItem::When(node) => {
                let loc = self.loc(node.syntax().range());
                let mut arms = Vec::new();
                if let Some(block) = node.block() {
                    let guard = self.expr(node.condition(), loc);
                    let body = self.nested(Some(block), pending);
                    arms.push(WhenArm { guard, body });
                }
                for arm in node.guard_arms() {
                    let arm_loc = self.loc(arm.syntax().range());
                    let guard = self.expr(arm.guard(), arm_loc);
                    let body = self.nested(arm.block(), pending);
                    arms.push(WhenArm { guard, body });
                }
                let otherwise = node
                    .else_branch()
                    .map(|branch| self.nested(branch.block(), pending));
                items.push(BodyItem::When {
                    arms,
                    otherwise,
                    loc,
                });
            }
        }
    }

    fn member(
        &mut self,
        member: Member<'p>,
        inline: bool,
        items: &mut Vec<BodyItem>,
        pending: &mut Vec<(ast::Block<'p>, BlockId)>,
    ) {
        match member {
            Member::Placement(node) => {
                if let Some(id) = self.node_placement(&node, inline, pending) {
                    items.push(BodyItem::Placement(id));
                }
            }
            Member::Paren(node) => match node.placement() {
                Some(placement) => {
                    if let Some(id) = self.node_placement(&placement, inline, pending) {
                        items.push(BodyItem::Placement(id));
                    }
                }
                None => self.dropped = true,
            },
            Member::Bundle(node) => {
                let range = node.syntax().range();
                if self.faults.touch(range.start..range.start + 1) {
                    self.dropped = true;
                    return;
                }
                let loc = self.loc(range.clone());
                let closed = node
                    .syntax()
                    .last_token()
                    .is_some_and(|token| token.kind() == SyntaxKind::RBracket);
                let faulty = !closed || self.faults.touch(range);
                let Some(members) = self.bundle_members(node.members(), pending) else {
                    self.dropped = true;
                    return;
                };
                let each = match node.bundle_for() {
                    Some(each) => {
                        let each_loc = self.loc(each.syntax().range());
                        let Some(members) = self.bundle_members(each.members(), pending) else {
                            self.dropped = true;
                            return;
                        };
                        let (names, iterable) = self.loop_head(&each.head(), each_loc);
                        Some(BundleFor {
                            names,
                            iterable,
                            members,
                            loc: each_loc,
                        })
                    }
                    None => None,
                };
                items.push(BodyItem::Bundle {
                    members,
                    each,
                    faulty,
                    loc,
                });
            }
        }
    }

    fn bundle_members(
        &mut self,
        members: impl Iterator<Item = Member<'p>>,
        pending: &mut Vec<(ast::Block<'p>, BlockId)>,
    ) -> Option<Vec<PlacementId>> {
        let mut ids = Vec::new();
        for member in members {
            let placement = match member {
                Member::Placement(node) => Some(node),
                Member::Paren(node) => node.placement(),
                Member::Bundle(_) => return None,
            };
            let placement = placement?;
            ids.push(self.node_placement(&placement, false, pending)?);
        }
        Some(ids)
    }

    fn nested(
        &mut self,
        node: Option<ast::Block<'p>>,
        pending: &mut Vec<(ast::Block<'p>, BlockId)>,
    ) -> BlockId {
        let Some(node) = node else {
            return self.absent_block();
        };
        let id = self.empty_block();
        pending.push((node, id));
        id
    }

    fn node_placement(
        &mut self,
        node: &ast::Placement<'p>,
        inline: bool,
        pending: &mut Vec<(ast::Block<'p>, BlockId)>,
    ) -> Option<PlacementId> {
        let Some(kind) = node.kind() else {
            self.dropped = true;
            return None;
        };
        if self.faults.touch(kind.range()) {
            self.dropped = true;
            return None;
        }
        Some(self.placement(node, inline, pending))
    }

    fn placement(
        &mut self,
        node: &ast::Placement<'p>,
        inline: bool,
        pending: &mut Vec<(ast::Block<'p>, BlockId)>,
    ) -> PlacementId {
        let loc = self.loc(head(node.syntax()));
        let default = node.default_arg().map(|arg| self.default_arg(&arg));
        let gate = node
            .gate()
            .map(|gate| self.expr(gate.condition(), self.loc(gate.syntax().range())));
        let attrs = self.attrs(node.attrs());
        let body = node.body().map(|body| self.nested(body.block(), pending));
        let placement = Placement {
            kind: self.ident(node.kind()),
            flags: self.flags(node.flags().as_ref().map(AstNode::syntax)),
            name: self.ident(node.name()),
            has_as: node.has_as(),
            default,
            gate,
            attrs,
            body,
            skipped_block: UnknownNames::default(),
            inline,
            loc,
        };
        let id = PlacementId(index(self.hir.placements.len()));
        self.hir.placements.push(placement);
        id
    }

    fn wire(&mut self, node: &ast::WirePlacement<'p>) -> Option<WireId> {
        let kind = self.ident(node.kind())?;
        let loc = self.loc(head(node.syntax()));
        let default = node.default_arg().map(|arg| self.default_arg(&arg));
        let gate = node
            .gate()
            .map(|gate| self.expr(gate.condition(), self.loc(gate.syntax().range())));
        let attrs = self.attrs(node.attrs());
        let destination = match node.destination() {
            Some(destination) => match destination.match_block() {
                Some(block) => self.match_block(&block),
                None => self.expr(destination.value(), self.loc(destination.syntax().range())),
            },
            None => self.push(ExprKind::Error, loc),
        };
        let wire = Wire {
            kind,
            flags: self.flags(node.flags().as_ref().map(AstNode::syntax)),
            name: self.ident(node.name()),
            default,
            gate,
            attrs,
            destination,
            loc,
        };
        let id = WireId(index(self.hir.wires.len()));
        self.hir.wires.push(wire);
        Some(id)
    }

    fn default_arg(&mut self, arg: &ast::DefaultArg<'p>) -> crate::semantics::ir::ExprId {
        let range = arg.syntax().range();
        let loc = self.loc(range.clone());
        if self.faults.touch(range.end..range.end + 1) {
            return self.push(ExprKind::Error, loc);
        }
        self.expr(arg.value(), loc)
    }

    fn attrs(&mut self, attrs: Option<ast::Attrs<'p>>) -> Vec<Attr> {
        let Some(attrs) = attrs else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in attrs.entries() {
            let loc = self.loc(entry.syntax().range());
            let Some(name) = self.ident(entry.name()) else {
                continue;
            };
            let value = if entry.is_negated() {
                AttrValue::Set(false)
            } else {
                match entry.value() {
                    Some(value) => AttrValue::Expr(self.expr(Some(value), loc)),
                    None if entry
                        .syntax()
                        .children_with_tokens()
                        .any(|element| element.kind() == SyntaxKind::Eq) =>
                    {
                        AttrValue::Expr(self.push(ExprKind::Error, loc))
                    }
                    None => AttrValue::Set(true),
                }
            };
            out.push(Attr { name, value, loc });
        }
        out
    }

    fn flags(&self, node: Option<&NodeView<'p>>) -> Vec<(char, Loc)> {
        let Some(node) = node else {
            return Vec::new();
        };
        let mut flags = Vec::new();
        for element in node.children_with_tokens() {
            let ElementView::Token(token) = element else {
                continue;
            };
            let range = token.range();
            for (offset, c) in token.text().char_indices() {
                let start = range.start + u32::try_from(offset).unwrap_or(0);
                let width = u32::try_from(c.len_utf8()).unwrap_or(1);
                flags.push((c, self.loc(start..start + width)));
            }
        }
        flags
    }

    pub(super) fn loop_head(
        &mut self,
        head: &ast::LoopHead<'p>,
        loc: Loc,
    ) -> (Vec<LoopVar>, Iterable) {
        let names = head
            .names()
            .into_iter()
            .map(|loop_name| LoopVar {
                name: self.ident(Some(loop_name.name)),
                ty: loop_name.ty.map(|ty| self.type_ref(&ty)),
            })
            .collect();
        (names, self.iterable(head.iterable(), loc))
    }

    fn iterable(&mut self, node: Option<ast::Iterable<'p>>, loc: Loc) -> Iterable {
        match node {
            Some(ast::Iterable::Range(range)) => {
                let range_loc = self.loc(range.syntax().range());
                Iterable::Range(
                    self.expr(range.start(), range_loc),
                    self.expr(range.end(), range_loc),
                )
            }
            Some(ast::Iterable::Expr(expr)) => Iterable::Values(self.expr(Some(expr), loc)),
            None => Iterable::Values(self.push(ExprKind::Error, loc)),
        }
    }
}
