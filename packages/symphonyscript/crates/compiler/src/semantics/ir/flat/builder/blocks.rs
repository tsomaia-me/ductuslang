use super::super::block_rec::BlockRec;
use super::super::file_rec::FileRec;
use super::super::item_rec::ItemRec;
use super::super::top_rec::TopRec;
use super::super::{NONE, index};
use super::ir_builder::IrBuilder;
use super::marks::Marks;
use super::ranges::{line_end, skipped_block};
use super::value::Value;
use crate::semantics::Internal;
use crate::syntax::{NodeView, SyntaxKind};

fn body_item(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Placement
            | SyntaxKind::WirePlacement
            | SyntaxKind::Line
            | SyntaxKind::ForBlock
            | SyntaxKind::WhenBlock
    )
}

fn top_item(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::LetDecl
            | SyntaxKind::CellDecl
            | SyntaxKind::FnDecl
            | SyntaxKind::PatternDecl
            | SyntaxKind::TypeDecl
            | SyntaxKind::Placement
            | SyntaxKind::WirePlacement
            | SyntaxKind::PackageDecl
            | SyntaxKind::Require
            | SyntaxKind::Use
            | SyntaxKind::Publish
    )
}

impl<'p> IrBuilder<'p, '_> {
    fn skip_block_of(&mut self, placement: u32, region: (u32, u32)) {
        if let Some(ItemRec::Placement(rec)) = self.ir.items.get_mut(placement as usize) {
            rec.skipped_block = Some(region);
        }
    }

    pub(super) fn block(&mut self, view: NodeView<'p>, base: usize) -> Value {
        let children: Vec<NodeView<'p>> = view.children().collect();
        let lets_start = index(self.ir.lets.len());
        for child in &children {
            if child.kind() == SyntaxKind::LocalLet
                && let Some(Value::Local(local)) = self.value_at(base, child)
            {
                self.local_let(local, NONE, (local.lo, local.hi));
            }
        }
        let lets = (lets_start, index(self.ir.lets.len()));
        let items_start = index(self.ir.block_items.len());
        let regions_start = index(self.ir.regions.len());
        let mut previous = None;
        let mut members = (index(self.ir.members.len()), index(self.ir.members.len()));
        let mut inline_regions = None;
        for child in &children {
            let kind = child.kind();
            if kind == SyntaxKind::ErrorNode {
                let range = child.range();
                match previous.take() {
                    Some(placement) if skipped_block(child) => {
                        self.skip_block_of(placement, (range.start, range.end));
                    }
                    _ => self.ir.regions.push((range.start, range.end)),
                }
            } else if body_item(kind) {
                let item = match self.value_at(base, child) {
                    Some(Value::Item(item)) => item,
                    _ => NONE,
                };
                self.ir.block_items.push(item);
                let dropped = self.ir.dropped(item);
                if dropped {
                    let range = child.range();
                    self.ir.regions.push((range.start, range.end));
                } else {
                    let errs = self.errs_after(base, child);
                    self.ir.regions.extend(errs);
                }
                previous = (kind == SyntaxKind::Placement && !dropped).then_some(item);
            } else if kind == SyntaxKind::Members
                && let Some(Value::Inline {
                    members: found,
                    regions,
                }) = self.value_at(base, child)
            {
                members = found;
                inline_regions = Some(regions);
            }
        }
        if let Some((from, to)) = inline_regions {
            let copied: Vec<(u32, u32)> = self
                .ir
                .regions
                .get(from as usize..to as usize)
                .unwrap_or_default()
                .to_vec();
            self.ir.regions.extend(copied);
        }
        self.ir.blocks.push(BlockRec {
            items: (items_start, index(self.ir.block_items.len())),
            members,
            regions: (regions_start, index(self.ir.regions.len())),
            lets,
        });
        Value::Block(index(self.ir.blocks.len() - 1))
    }

    pub(super) fn error_node(&mut self, view: NodeView<'p>, marks: Marks) -> Value {
        marks.truncate(self.ir);
        self.errs.truncate(marks.errs as usize);
        let range = view.range();
        self.errs.push((range.start, range.end));
        Value::Nothing
    }

    pub(super) fn file(
        &mut self,
        view: NodeView<'p>,
        base: usize,
        _file: &mut FileRec,
    ) -> Result<(u32, u32), Internal> {
        let regions_start = index(self.ir.regions.len());
        let mut previous = None;
        for child in view.children() {
            let kind = child.kind();
            if kind == SyntaxKind::ErrorNode {
                let range = child.range();
                match previous.take() {
                    Some(placement) if skipped_block(&child) => {
                        self.skip_block_of(placement, (range.start, range.end));
                    }
                    _ => self.ir.regions.push((range.start, range.end)),
                }
            } else if top_item(kind) {
                let errs = self.errs_after(base, &child);
                self.ir.regions.extend(errs);
                if kind != SyntaxKind::Placement
                    && let Some(fault) = self.first_in(child.range())
                {
                    self.ir.regions.push((fault, line_end(&child)));
                }
                let top = match self.value_at(base, &child) {
                    Some(Value::Top(top)) => top,
                    Some(Value::Item(item)) if kind == SyntaxKind::Placement => {
                        TopRec::Placement(item)
                    }
                    _ => TopRec::Skip,
                };
                self.ir.top.push(top);
                previous = match top {
                    TopRec::Placement(item) => Some(item),
                    TopRec::Let(_)
                    | TopRec::Cell(_)
                    | TopRec::Fn(_)
                    | TopRec::Pattern(_)
                    | TopRec::Placeholder { .. }
                    | TopRec::Drawn(_)
                    | TopRec::Skip => None,
                };
            } else {
                previous = None;
            }
        }
        Ok((regions_start, index(self.ir.regions.len())))
    }
}
