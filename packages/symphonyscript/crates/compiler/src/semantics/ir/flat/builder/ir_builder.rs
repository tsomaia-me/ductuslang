use std::ops::Range;

use symphonyscript_core::Vocabulary;
use unicode_normalization::UnicodeNormalization;

use super::super::drawn::Drawn;
use super::super::file_rec::FileRec;
use super::super::flat_ir::FlatIr;
use super::super::name_kind::NameKind;
use super::super::name_range::NameRange;
use super::super::op::Op;
use super::super::{NONE, index};
use super::marks::Marks;
use super::value::Value;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::Faults;
use crate::syntax::ast::{AstNode, Name};
use crate::syntax::{NodeView, Parsed, SyntaxKind};

pub(super) struct IrBuilder<'p, 'i> {
    pub ir: &'i mut FlatIr,
    pub parsed: &'p Parsed,
    pub vocabulary: &'p Vocabulary,
    pub faults: Faults,
    line_faults: Vec<(u32, u32)>,
    stack: Vec<(u32, Value, Marks)>,
    pub staged: Vec<(u32, Value)>,
    pub errs: Vec<(u32, u32)>,
    own: Vec<(u32, u32)>,
    children: Vec<u32>,
    scratch: String,
}

pub(crate) fn build_flat_file(
    ir: &mut FlatIr,
    path: &str,
    parsed: &Parsed,
    vocabulary: &Vocabulary,
) -> Result<(), Internal> {
    let path_start = index(ir.texts.len());
    ir.texts.push_str(path);
    let first_node = ir.end();
    ir.file_first.push(first_node);
    let names_start = ir.names.len();
    let top_start = index(ir.top.len());
    let mut file = FileRec {
        path: (path_start, index(ir.texts.len())),
        top: (top_start, top_start),
        regions: (index(ir.regions.len()), index(ir.regions.len())),
        names: (index(ir.name_order.len()), index(ir.name_order.len())),
        nodes: (first_node, first_node),
    };
    if let Some(root) = parsed.file() {
        let mut builder = IrBuilder {
            ir: &mut *ir,
            parsed,
            vocabulary,
            faults: Faults::new(&parsed.diagnostics, &parsed.detached, root.syntax()),
            line_faults: line_faults(parsed),
            stack: Vec::new(),
            staged: Vec::new(),
            errs: Vec::new(),
            own: Vec::new(),
            children: Vec::new(),
            scratch: String::new(),
        };
        file.regions = builder.run(&mut file)?;
    }
    let order_start = ir.name_order.len();
    ir.name_order
        .extend((names_start..ir.names.len()).map(index));
    let names = &ir.names;
    if let Some(order) = ir.name_order.get_mut(order_start..) {
        order.sort_by_key(|name| names.get(*name as usize).map_or(0, |entry| entry.lo));
    }
    file.names = (index(order_start), index(ir.name_order.len()));
    file.top.1 = index(ir.top.len());
    file.nodes.1 = ir.end();
    ir.files.push(file);
    Ok(())
}

impl<'p> IrBuilder<'p, '_> {
    fn run(&mut self, file: &mut FileRec) -> Result<(u32, u32), Internal> {
        let tree = self.parsed.tree();
        let count = u32::try_from(tree.len()).unwrap_or(u32::MAX);
        let mut regions = (index(self.ir.regions.len()), index(self.ir.regions.len()));
        for node in 0..count {
            CountedWalk::IrBuilderStep.step();
            self.children.clear();
            let mut child = tree.last_child(node);
            while let Some(found) = child {
                self.children.push(found);
                child = tree.previous_sibling(node, found);
            }
            self.children.reverse();
            let base = self
                .stack
                .len()
                .checked_sub(self.children.len())
                .ok_or(Internal::missed("IR builder value"))?;
            let aligned = self
                .stack
                .get(base..)
                .unwrap_or_default()
                .iter()
                .map(|slot| slot.0)
                .eq(self.children.iter().copied());
            if !aligned {
                return Err(Internal::missed("IR builder value"));
            }
            let mut marks = match self.stack.get(base) {
                Some(first) => first.2,
                None => Marks::of(self.ir, self.staged.len(), self.errs.len()),
            };
            marks.empty_type = self
                .stack
                .get(base..)
                .unwrap_or_default()
                .iter()
                .any(|slot| slot.2.empty_type);
            self.own_names(node);
            let view = NodeView::new(self.parsed, node);
            let value = if view.kind() == SyntaxKind::File {
                regions = self.file(view, base, file)?;
                Value::Nothing
            } else {
                self.finish(view, base, marks)?
            };
            if !matches!(value, Value::Staged { .. }) {
                self.staged.truncate(marks.staged as usize);
            }
            self.stack.truncate(base);
            marks.empty_type |= view.kind() == SyntaxKind::TypeRef && view.range().is_empty();
            self.stack.push((node, value, marks));
        }
        Ok(regions)
    }

    fn own_names(&mut self, node: u32) {
        self.own.clear();
        let tree = self.parsed.tree();
        let (first, end) = tree.tokens(node);
        let mut token = first;
        for at in 0..=self.children.len() {
            let (limit, next) = match self.children.get(at) {
                Some(child) => tree.tokens(*child),
                None => (end, end),
            };
            for gap in token..limit {
                self.own_name(gap);
            }
            token = next;
        }
    }

    fn own_name(&mut self, token: u32) {
        let tokens = self.parsed.tokens();
        if tokens.kind(token as usize) != Some(SyntaxKind::Ident) {
            return;
        }
        let Some(range) = tokens.range(token as usize) else {
            return;
        };
        let text = self
            .parsed
            .text()
            .get(range.start as usize..range.end as usize)
            .unwrap_or_default();
        let sym = if text.is_ascii() {
            self.ir.symbols.intern(text)
        } else {
            self.scratch.clear();
            self.scratch.extend(text.nfc());
            self.ir.symbols.intern(&self.scratch)
        };
        let faulty = self.faults.touch(range.clone());
        self.own.push((token, index(self.ir.names.len())));
        self.ir.names.push(NameRange {
            sym,
            lo: range.start,
            hi: range.end,
            kind: NameKind::Unclassified,
            faulty,
        });
    }

    pub fn name_of(&mut self, name: Option<Name<'_>>, kind: NameKind) -> u32 {
        let Some(name) = name else {
            return NONE;
        };
        let token = name.token().index();
        let found = self
            .own
            .iter()
            .find(|(own, _)| *own == token)
            .map_or(NONE, |(_, entry)| *entry);
        if let Some(entry) = self.ir.names.get_mut(found as usize) {
            entry.kind = kind;
        }
        found
    }

    pub fn named(&self, name: u32) -> bool {
        self.ir.name(name).is_some_and(|entry| !entry.faulty)
    }

    pub fn value_at(&self, base: usize, node: &NodeView<'_>) -> Option<Value> {
        let slots = self.stack.get(base..)?;
        let at = slots
            .binary_search_by_key(&node.index(), |slot| slot.0)
            .ok()?;
        slots.get(at).map(|slot| slot.1)
    }

    pub fn staged_at(&self, block: Option<Value>, node: &NodeView<'_>) -> Option<Value> {
        let Some(Value::Staged { start, end }) = block else {
            return None;
        };
        let slots = self.staged.get(start as usize..end as usize)?;
        let at = slots
            .binary_search_by_key(&node.index(), |slot| slot.0)
            .ok()?;
        slots.get(at).map(|slot| slot.1)
    }

    pub fn stage(&mut self, base: usize, marks: Marks) -> Value {
        let values: Vec<(u32, Value)> = self
            .stack
            .get(base..)
            .unwrap_or_default()
            .iter()
            .map(|slot| (slot.0, slot.1))
            .collect();
        self.staged.truncate(marks.staged as usize);
        let start = index(self.staged.len());
        self.staged.extend(values);
        Value::Staged {
            start,
            end: index(self.staged.len()),
        }
    }

    pub fn emit(&mut self, op: Op, operands: (u32, u32, u32), span: (u32, u32)) -> u32 {
        CountedWalk::IrNode.step();
        let id = self.ir.end();
        self.ir.op.push(op);
        self.ir.a.push(operands.0);
        self.ir.b.push(operands.1);
        self.ir.c.push(operands.2);
        self.ir.lo.push(span.0);
        self.ir.hi.push(span.1);
        id
    }

    pub fn bad(&mut self, span: (u32, u32)) -> u32 {
        let at = index(self.ir.drawn.len());
        self.emit(Op::Bad, (at, at, NONE), span)
    }

    pub fn bad_drawn(&mut self, span: (u32, u32), from: u32) -> u32 {
        let to = index(self.ir.drawn.len());
        self.emit(Op::Bad, (from, to, NONE), span)
    }

    pub fn draw(&mut self, code: Code, range: (u32, u32), summary: &str) {
        if self.faults.touch(range.0..range.1) {
            return;
        }
        let start = index(self.ir.texts.len());
        self.ir.texts.push_str(summary);
        self.ir.drawn.push(Drawn {
            code,
            lo: range.0,
            hi: range.1,
            summary: (start, index(self.ir.texts.len())),
        });
    }

    pub fn draw_unbuilt(&mut self, range: (u32, u32), construct: &str) {
        self.draw(
            Code::UnbuiltConstruct,
            range,
            &format!("{construct} is a construct this compiler does not build"),
        );
    }

    pub fn drawn_mark(&self) -> u32 {
        index(self.ir.drawn.len())
    }

    pub fn force(&mut self, value: Option<Value>) -> Option<u32> {
        match value? {
            Value::Expr(id) => Some(id),
            Value::Pending(pending) => Some(self.force_pending(pending)),
            Value::Nothing
            | Value::Ty { .. }
            | Value::Arm { .. }
            | Value::Else(_)
            | Value::Local(_)
            | Value::Named { .. }
            | Value::Range { .. }
            | Value::Head(_)
            | Value::Block(_)
            | Value::Item(_)
            | Value::Attr(_)
            | Value::Guard { .. }
            | Value::Each { .. }
            | Value::Staged { .. }
            | Value::Members(_)
            | Value::Param { .. }
            | Value::Field { .. }
            | Value::Top(_)
            | Value::Inline { .. } => None,
        }
    }

    pub fn expr_of<N: AstNode<'p>>(
        &mut self,
        base: usize,
        child: Option<N>,
        fallback: (u32, u32),
    ) -> u32 {
        let value = child.and_then(|child| self.value_at(base, child.syntax()));
        match self.force(value) {
            Some(id) => id,
            None => self.bad(fallback),
        }
    }

    pub fn or_bad(&mut self, node: u32, fallback: (u32, u32)) -> u32 {
        if node == NONE {
            self.bad(fallback)
        } else {
            node
        }
    }

    pub fn commit_type(&mut self, value: Option<Value>) -> u32 {
        let Some(Value::Ty { root, base }) = value else {
            return NONE;
        };
        if let Ok((Op::Bad, _, _, construct)) = self.ir.node(base)
            && construct != NONE
            && let Ok(span) = self.ir.span(base)
            && let Some(text) = super::constructs::CONSTRUCTS.get(construct as usize)
        {
            let from = self.drawn_mark();
            self.draw_unbuilt(span, text);
            let to = self.drawn_mark();
            self.set_node(base, Op::Bad, (from, to, NONE));
        }
        root
    }

    pub fn set_node(&mut self, node: u32, op: Op, operands: (u32, u32, u32)) {
        let Some(local) = node.checked_sub(self.ir.base).map(|local| local as usize) else {
            return;
        };
        if let Some(slot) = self.ir.op.get_mut(local) {
            *slot = op;
        }
        if let Some(slot) = self.ir.a.get_mut(local) {
            *slot = operands.0;
        }
        if let Some(slot) = self.ir.b.get_mut(local) {
            *slot = operands.1;
        }
        if let Some(slot) = self.ir.c.get_mut(local) {
            *slot = operands.2;
        }
    }

    pub fn set_span(&mut self, node: u32, span: (u32, u32)) {
        let Some(local) = node.checked_sub(self.ir.base).map(|local| local as usize) else {
            return;
        };
        if let Some(slot) = self.ir.lo.get_mut(local) {
            *slot = span.0;
        }
        if let Some(slot) = self.ir.hi.get_mut(local) {
            *slot = span.1;
        }
    }

    pub fn touch(&self, range: Range<u32>) -> bool {
        self.faults.touch(range)
    }

    pub fn fault_reach(&self, last: u32) -> u32 {
        let count = self
            .line_faults
            .partition_point(|(start, _)| *start <= last);
        count
            .checked_sub(1)
            .and_then(|at| self.line_faults.get(at))
            .map_or(0, |(_, reach)| *reach)
    }

    pub fn first_in(&self, range: Range<u32>) -> Option<u32> {
        self.faults.first_in(range)
    }

    pub fn children_of(&self, base: usize) -> Vec<(u32, Value, Marks)> {
        self.stack.get(base..).unwrap_or_default().to_vec()
    }

    pub fn errs_after(&self, base: usize, node: &NodeView<'_>) -> Vec<(u32, u32)> {
        let slots = self.stack.get(base..).unwrap_or_default();
        let Ok(at) = slots.binary_search_by_key(&node.index(), |slot| slot.0) else {
            return Vec::new();
        };
        let from = slots.get(at).map_or(0, |slot| slot.2.errs as usize);
        let to = slots
            .get(at + 1)
            .map_or(self.errs.len(), |slot| slot.2.errs as usize);
        self.errs.get(from..to).unwrap_or_default().to_vec()
    }
}

pub(super) fn span(node: &NodeView<'_>) -> (u32, u32) {
    let range = node.range();
    (range.start, range.end)
}

fn line_faults(parsed: &Parsed) -> Vec<(u32, u32)> {
    let mut faults: Vec<(u32, u32)> = parsed
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let range = &diagnostic.span.range;
            if parsed.detached.contains(range)
                && let Some(break_at) = break_before(parsed.text(), range.start)
            {
                return (break_at, break_at.saturating_add(1));
            }
            (range.start, range.end.max(range.start.saturating_add(1)))
        })
        .collect();
    faults.sort_unstable();
    let mut furthest = 0;
    for fault in &mut faults {
        furthest = furthest.max(fault.1);
        fault.1 = furthest;
    }
    faults
}

fn break_before(text: &str, start: u32) -> Option<u32> {
    let before = text.get(..start as usize)?;
    let break_at = before.rfind('\n')?;
    before
        .get(break_at + 1..)
        .is_some_and(|indent| indent.chars().all(char::is_whitespace))
        .then(|| u32::try_from(break_at).ok())
        .flatten()
}
