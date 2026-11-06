use super::super::attr_entry_rec::AttrEntryRec;
use super::super::attr_form::AttrForm;
use super::super::binder_kind::BinderKind;
use super::super::bundle_for_rec::BundleForRec;
use super::super::bundle_rec::BundleRec;
use super::super::flag_rec::FlagRec;
use super::super::for_block_rec::ForBlockRec;
use super::super::item_rec::ItemRec;
use super::super::name_kind::NameKind;
use super::super::placement_rec::PlacementRec;
use super::super::when_rec::WhenRec;
use super::super::wire_rec::WireRec;
use super::super::{ABSENT, NONE, index};
use super::ir_builder::{IrBuilder, span};
use super::marks::Marks;
use super::ranges::head;
use super::value::Value;
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{ElementView, NodeView, SyntaxKind};

impl<'p> IrBuilder<'p, '_> {
    fn expr_value(&mut self, base: usize, view: Option<NodeView<'_>>) -> u32 {
        let value = view.and_then(|view| self.value_at(base, &view));
        self.force(value).unwrap_or(NONE)
    }

    fn flags_of(&mut self, flags: Option<ast::Flags<'_>>) -> (u32, u32) {
        let start = index(self.ir.flags.len());
        if let Some(flags) = flags {
            for element in flags.syntax().children_with_tokens() {
                let ElementView::Token(token) = element else {
                    continue;
                };
                let range = token.range();
                for (offset, flag) in token.text().char_indices() {
                    let lo = range.start + u32::try_from(offset).unwrap_or(0);
                    let width = u32::try_from(flag.len_utf8()).unwrap_or(1);
                    self.ir.flags.push(FlagRec {
                        flag,
                        lo,
                        hi: lo + width,
                    });
                }
            }
        }
        (start, index(self.ir.flags.len()))
    }

    fn attrs_of(&mut self, base: usize, attrs: Option<ast::Attrs<'_>>) -> ((u32, u32), bool) {
        let Some(attrs) = attrs else {
            let at = index(self.ir.attr_entries.len());
            return ((at, at), false);
        };
        let range = attrs.syntax().range();
        let cut = self.first_in(range).is_some();
        match self.value_at(base, attrs.syntax()) {
            Some(Value::Members(entries)) => (entries, cut),
            _ => {
                let at = index(self.ir.attr_entries.len());
                ((at, at), cut)
            }
        }
    }

    pub(super) fn placement(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::Placement::cast(view)?;
        let kind = self.name_of(node.kind(), NameKind::ItemKind);
        let as_name = self.name_of(node.name(), NameKind::AsName);
        let flags = self.flags_of(node.flags());
        let default = self.expr_value(base, node.default_arg().map(|arg| *arg.syntax()));
        let gate = self.expr_value(base, node.gate().map(|gate| *gate.syntax()));
        let (attrs, attrs_cut) = self.attrs_of(base, node.attrs());
        let body = match node.body() {
            Some(body) => match self.value_at(base, body.syntax()) {
                Some(Value::Block(block)) => block,
                _ => ABSENT,
            },
            None => NONE,
        };
        let at = head(&view);
        let faulty_line = self.faulty_line(&view);
        self.ir.items.push(ItemRec::Placement(PlacementRec {
            kind,
            flags,
            as_name,
            has_as: node.has_as(),
            default,
            gate,
            attrs,
            attrs_cut,
            body,
            skipped_block: None,
            inline: false,
            lo: at.0,
            hi: at.1,
            faulty_line,
        }));
        Some(Value::Item(index(self.ir.items.len() - 1)))
    }

    pub(super) fn wire(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::WirePlacement::cast(view)?;
        let kind = self.name_of(node.kind(), NameKind::ItemKind);
        let as_name = self.name_of(node.name(), NameKind::AsName);
        let flags = self.flags_of(node.flags());
        let at = head(&view);
        let default = self.expr_value(base, node.default_arg().map(|arg| *arg.syntax()));
        let gate = self.expr_value(base, node.gate().map(|gate| *gate.syntax()));
        let (attrs, attrs_cut) = self.attrs_of(base, node.attrs());
        let destination = self.expr_value(base, node.destination().map(|dest| *dest.syntax()));
        let destination = self.or_bad(destination, at);
        self.ir.items.push(ItemRec::Wire(WireRec {
            kind,
            flags,
            as_name,
            default,
            gate,
            attrs,
            attrs_cut,
            destination,
            lo: at.0,
            hi: at.1,
        }));
        Some(Value::Item(index(self.ir.items.len() - 1)))
    }

    pub(super) fn default_arg(
        &mut self,
        view: NodeView<'p>,
        base: usize,
        marks: Marks,
    ) -> Option<Value> {
        let node = ast::DefaultArg::cast(view)?;
        let whole = span(&view);
        if self.touch(whole.1..whole.1 + 1) {
            return Some(Value::Expr(self.dead(marks, whole)));
        }
        Some(Value::Expr(self.expr_of(base, node.value(), whole)))
    }

    pub(super) fn attr_entry(
        &mut self,
        view: NodeView<'p>,
        base: usize,
        marks: Marks,
    ) -> Option<Value> {
        let entry = ast::AttrEntry::cast(view)?;
        let whole = span(&view);
        let name = self.name_of(entry.name(), NameKind::AttributeName);
        if !self.named(name) {
            marks.truncate(self.ir);
            return Some(Value::Nothing);
        }
        let binds = view
            .children_with_tokens()
            .any(|element| element.kind() == SyntaxKind::FatArrow);
        let form = if binds {
            marks.truncate(self.ir);
            let from = self.drawn_mark();
            if entry.value().is_some() {
                self.draw_unbuilt(whole, "the binding `=>` of an attribute (SPEC §7.6)");
            }
            AttrForm::Value(self.bad_drawn(whole, from))
        } else if entry.is_negated() {
            marks.truncate(self.ir);
            AttrForm::Set(false)
        } else {
            match entry.value() {
                Some(value) => AttrForm::Value(self.expr_of(base, Some(value), whole)),
                None if view
                    .children_with_tokens()
                    .any(|element| element.kind() == SyntaxKind::Eq) =>
                {
                    AttrForm::Value(self.bad(whole))
                }
                None => AttrForm::Set(true),
            }
        };
        Some(Value::Attr(AttrEntryRec {
            name,
            form,
            lo: whole.0,
            hi: whole.1,
        }))
    }

    pub(super) fn attrs(&mut self, base: usize) -> Value {
        let entries: Vec<AttrEntryRec> = self
            .children_of(base)
            .into_iter()
            .filter_map(|(_, value, _)| match value {
                Value::Attr(entry) => Some(entry),
                _ => None,
            })
            .collect();
        let start = index(self.ir.attr_entries.len());
        self.ir.attr_entries.extend(entries);
        Value::Members((start, index(self.ir.attr_entries.len())))
    }

    pub(super) fn member_items(
        &mut self,
        view: &NodeView<'_>,
        base: usize,
        inline: bool,
    ) -> (u32, u32) {
        let start = index(self.ir.members.len());
        let mut position = 0usize;
        for child in view.children() {
            if !matches!(
                child.kind(),
                SyntaxKind::Placement | SyntaxKind::ParenPlacement | SyntaxKind::Bundle
            ) {
                continue;
            }
            let item = match self.value_at(base, &child) {
                Some(Value::Item(item)) => item,
                _ => NONE,
            };
            if inline && position > 0 {
                self.set_inline(item);
            }
            position += 1;
            self.ir.members.push(item);
        }
        (start, index(self.ir.members.len()))
    }

    fn set_inline(&mut self, item: u32) {
        let target = match self.ir.items.get(item as usize) {
            Some(ItemRec::Paren(inner)) => *inner,
            _ => item,
        };
        if let Some(ItemRec::Placement(placement)) = self.ir.items.get_mut(target as usize) {
            placement.inline = true;
        }
    }

    pub(super) fn members_value(&mut self, view: NodeView<'p>, base: usize) -> Value {
        let members = self.member_items(&view, base, true);
        let start = index(self.ir.regions.len());
        for child in view.children() {
            if !matches!(
                child.kind(),
                SyntaxKind::Placement | SyntaxKind::ParenPlacement | SyntaxKind::Bundle
            ) {
                continue;
            }
            let item = match self.value_at(base, &child) {
                Some(Value::Item(item)) => item,
                _ => NONE,
            };
            if self.ir.dropped(item) {
                let range = child.range();
                self.ir.regions.push((range.start, range.end));
            } else {
                let errs = self.errs_after(base, &child);
                self.ir.regions.extend(errs);
            }
        }
        Value::Inline {
            members,
            regions: (start, index(self.ir.regions.len())),
        }
    }

    pub(super) fn paren_placement(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::ParenPlacement::cast(view)?;
        let inner = match node
            .placement()
            .and_then(|placement| self.value_at(base, placement.syntax()))
        {
            Some(Value::Item(item)) => item,
            _ => NONE,
        };
        self.ir.items.push(ItemRec::Paren(inner));
        Some(Value::Item(index(self.ir.items.len() - 1)))
    }

    pub(super) fn bundle(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::Bundle::cast(view)?;
        let whole = span(&view);
        let start_touched = self.touch(whole.0..whole.0 + 1);
        let closed = view
            .last_token()
            .is_some_and(|token| token.kind() == SyntaxKind::RBracket);
        let faulty = !closed || self.touch(whole.0..whole.1);
        let members = self.member_items(&view, base, false);
        let each = match node
            .bundle_for()
            .and_then(|each| self.value_at(base, each.syntax()))
        {
            Some(Value::Each {
                head,
                members,
                lo,
                hi,
            }) => Some(BundleForRec {
                head,
                members,
                lo,
                hi,
            }),
            _ => None,
        };
        self.ir.items.push(ItemRec::Bundle(BundleRec {
            members,
            each,
            faulty,
            start_touched,
            lo: whole.0,
            hi: whole.1,
        }));
        Some(Value::Item(index(self.ir.items.len() - 1)))
    }

    pub(super) fn bundle_for(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::BundleFor::cast(view)?;
        let whole = span(&view);
        let members = self.member_items(&view, base, false);
        let head = self.iteration(&node.head(), base, BinderKind::BundleLoop, whole);
        Some(Value::Each {
            head,
            members,
            lo: whole.0,
            hi: whole.1,
        })
    }

    pub(super) fn for_block(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::ForBlock::cast(view)?;
        let whole = span(&view);
        let head = self.iteration(&node.head(), base, BinderKind::PlacementLoop, whole);
        let body = self.block_of(base, node.block());
        self.ir.items.push(ItemRec::For(ForBlockRec {
            head,
            body,
            lo: whole.0,
            hi: whole.1,
        }));
        Some(Value::Item(index(self.ir.items.len() - 1)))
    }

    fn block_of(&self, base: usize, block: Option<ast::Block<'_>>) -> u32 {
        match block.and_then(|block| self.value_at(base, block.syntax())) {
            Some(Value::Block(block)) => block,
            _ => ABSENT,
        }
    }

    pub(super) fn when_block(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::WhenBlock::cast(view)?;
        let whole = span(&view);
        let mut arms = Vec::new();
        if let Some(block) = node.block() {
            let guard = self.expr_of(base, node.condition(), whole);
            let block = self.block_of(base, Some(block));
            arms.push((guard, block));
        }
        for arm in node.guard_arms() {
            if let Some(Value::Guard { guard, block }) = self.value_at(base, arm.syntax()) {
                arms.push((guard, block));
            }
        }
        let otherwise = match node.else_branch() {
            Some(branch) => match self.value_at(base, branch.syntax()) {
                Some(Value::Block(block)) => block,
                _ => ABSENT,
            },
            None => NONE,
        };
        let start = index(self.ir.arms.len());
        self.ir.arms.extend(arms);
        self.ir.items.push(ItemRec::When(WhenRec {
            arms: (start, index(self.ir.arms.len())),
            otherwise,
            lo: whole.0,
            hi: whole.1,
        }));
        Some(Value::Item(index(self.ir.items.len() - 1)))
    }

    pub(super) fn guard_arm(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::GuardArm::cast(view)?;
        let whole = span(&view);
        let guard = self.expr_of(base, node.guard(), whole);
        let block = self.block_of(base, node.block());
        Some(Value::Guard { guard, block })
    }

    pub(super) fn when_else(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::WhenElse::cast(view)?;
        Some(Value::Block(self.block_of(base, node.block())))
    }

    pub(super) fn destination(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::Destination::cast(view)?;
        let whole = span(&view);
        let value = match node.match_block() {
            Some(block) => self.expr_of(base, Some(block), whole),
            None => self.expr_of(base, node.value(), whole),
        };
        Some(Value::Expr(value))
    }

    pub(super) fn body(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::Body::cast(view)?;
        Some(Value::Block(self.block_of(base, node.block())))
    }
}
