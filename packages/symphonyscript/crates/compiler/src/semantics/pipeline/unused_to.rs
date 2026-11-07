use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::paths::End;
use super::reason::Reason;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{AttrForm, NONE, Op, WireRec};

impl Core<'_> {
    pub fn unrun_to_paths(&mut self, wire: WireRec) -> Stage<()> {
        let paths = self.wire_to_paths(wire)?;
        if paths.is_empty() {
            return Ok(Ok(()));
        }
        let candidates = match self.written_candidates(wire.destination)? {
            Ok(Some(candidates)) => candidates,
            Ok(None) => return Ok(Ok(())),
            Err(declined) => return Ok(Err(declined)),
        };
        for path in paths {
            let outcome = if let [row] = candidates.as_slice() {
                let segments = self.to_segments(path)?;
                self.candidate_attr(path, &segments, *row, false)?
                    .map(|_| ())
            } else {
                self.written_choice_read(path, &candidates)?
            };
            if let Err(declined) = outcome {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    fn wire_to_paths(&self, wire: WireRec) -> Result<Vec<u32>, Internal> {
        let mut roots = vec![wire.default, wire.gate];
        for entry in self
            .ir
            .attr_entries
            .get(wire.attrs.0 as usize..wire.attrs.1 as usize)
            .unwrap_or_default()
        {
            if let AttrForm::Value(value) = entry.form {
                roots.push(value);
            }
        }
        let mut paths = Vec::new();
        for root in roots.into_iter().filter(|root| *root != NONE) {
            for node in self.deep_postorder(root)? {
                if self.ir.node(node)?.0 != Op::Field || self.is_field_receiver(node) {
                    continue;
                }
                let mut current = node;
                while self.ir.node(current)?.0 == Op::Field {
                    CountedWalk::CorePathStep.step();
                    current = self.ir.node(current)?.1;
                }
                if self.ir.node(current)?.0 == Op::To {
                    paths.push(node);
                }
            }
        }
        Ok(paths)
    }

    fn to_segments(&self, path: u32) -> Result<Vec<u32>, Internal> {
        let mut segments = Vec::new();
        let mut current = path;
        loop {
            CountedWalk::CorePathStep.step();
            let (op, receiver, field, _) = self.ir.node(current)?;
            if op != Op::Field {
                break;
            }
            segments.push(field);
            current = receiver;
        }
        segments.reverse();
        Ok(segments)
    }

    fn written_candidates(&self, destination: u32) -> Stage<Option<Vec<u32>>> {
        let mut candidates = Vec::new();
        let mut stack = vec![destination];
        while let Some(node) = stack.pop() {
            CountedWalk::CoreWireDestination.step();
            let (op, a, _, _) = self.ir.node(node)?;
            match op {
                Op::If => stack.extend(self.operands(node)?.into_iter().skip(1).rev()),
                Op::Array => stack.extend(self.operands(node)?.into_iter().rev()),
                Op::Index => stack.push(a),
                Op::Name => match self.bound(node)? {
                    DeclRef::Instance(item) => {
                        let row = self.row_of_item.get(item as usize).copied().unwrap_or(NONE);
                        if row == NONE && !self.use_links.is_empty() {
                            return Ok(Err(Declined(Reason::BodyItem)));
                        }
                        if row == NONE {
                            return Ok(Ok(None));
                        }
                        if !candidates.contains(&row) {
                            candidates.push(row);
                        }
                    }
                    DeclRef::Param(_) if !self.use_links.is_empty() => {
                        return Ok(Err(Declined(Reason::BodyItem)));
                    }
                    DeclRef::None | DeclRef::Duplicate | DeclRef::Param(_) | DeclRef::Cell(_) => {
                        return Ok(Ok(None));
                    }
                    DeclRef::Let(_) | DeclRef::Fn(_) | DeclRef::Pattern(_) => {
                        return Ok(Err(Declined(Reason::BodyItem)));
                    }
                },
                Op::Int | Op::Float | Op::Bool | Op::Str | Op::Variant => return Ok(Ok(None)),
                _ => return Ok(Err(Declined(Reason::BodyItem))),
            }
        }
        Ok(Ok(Some(candidates)))
    }

    fn written_choice_read(&mut self, path: u32, candidates: &[u32]) -> Stage<()> {
        let segments = self.to_segments(path)?;
        let Some((last, steps)) = segments.split_last() else {
            return Err(Internal::missed("a `to` path's segment"));
        };
        let text = segments
            .iter()
            .map(|segment| self.ir.name_text(*segment).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(".");
        let last = *self.ir.name(*last).ok_or(Internal::missed("IR name"))?;
        let (mut instances, mut lacking) = (0, Vec::new());
        'candidates: for candidate in candidates {
            CountedWalk::CorePathStep.step();
            let mut target = *candidate;
            for step in steps {
                let entry = *self.ir.name(*step).ok_or(Internal::missed("IR name"))?;
                match self.to_step(End::Row(target), entry.sym)? {
                    Some(End::Row(child)) => target = child,
                    Some(_) => return Ok(Err(Declined(Reason::Member))),
                    None if self.may_name_any(target)? => return Ok(Ok(())),
                    None => {
                        lacking.push(*candidate);
                        continue 'candidates;
                    }
                }
            }
            if self.to_step(End::Row(target), last.sym)?.is_some() {
                instances += 1;
                continue;
            }
            let kind = self
                .instances
                .get(target as usize)
                .ok_or(Internal::missed("row"))?
                .kind;
            let name = self.ir.symbols.text(last.sym);
            let declared = self
                .vocabulary
                .node_kind(kind)
                .is_some_and(|decl| decl.attrs.iter().any(|attr| attr.name == name));
            if declared || self.may_name_any(target)? {
                return Ok(Ok(()));
            }
            lacking.push(*candidate);
        }
        if instances == candidates.len() {
            let span = self.span(self.file_of(path), last.lo, last.hi);
            self.draw(
                Code::MisplacedTo,
                span,
                format!(
                    "`to.{text}` ends at an instance or a wire; a `to` path ends at an attribute"
                ),
            );
            return Ok(Ok(()));
        }
        let mut related = Vec::with_capacity(lacking.len());
        for candidate in &lacking {
            let owner = self.row_owner(*candidate)?;
            related.push((
                self.row_span(*candidate)?,
                format!("{owner} has no `to.{text}`"),
            ));
        }
        let summary = match (instances, candidates, lacking.first()) {
            (0, [first, second, ..], _) => format!(
                "no candidate of the destination, {} and {} among them, has `to.{text}`",
                self.row_owner(*first)?,
                self.row_owner(*second)?
            ),
            (_, _, Some(first)) => format!(
                "no candidate of the destination has the attribute `to.{text}`: {} has no such path, and the others end it at an instance or a wire",
                self.row_owner(*first)?
            ),
            _ => return Ok(Ok(())),
        };
        let span = self.node_span(path)?;
        self.draw_related(Code::UnknownAttribute, span, related, summary);
        Ok(Ok(()))
    }
}

impl Core<'_> {
    pub fn runtime_targets(&self, destination: u32) -> Result<Option<Vec<u32>>, Internal> {
        let (op, a, b, c) = self.ir.node(destination)?;
        let (test, arms) = match op {
            Op::Index if self.ir.node(a)?.0 == Op::Array => (b, self.operands(a)?),
            Op::If => (a, vec![b, c]),
            _ => return Ok(None),
        };
        if self.ir.node(test)?.0 != Op::Name || !matches!(self.bound(test)?, DeclRef::Cell(_)) {
            return Ok(None);
        }
        let mut rows = Vec::with_capacity(arms.len());
        for arm in arms {
            CountedWalk::CoreWireDestination.step();
            if self.ir.node(arm)?.0 != Op::Name {
                return Ok(None);
            }
            let DeclRef::Instance(item) = self.bound(arm)? else {
                return Ok(None);
            };
            match self.row_of_item.get(item as usize).copied() {
                Some(row) if row != NONE => rows.push(row),
                Some(_) | None => return Ok(None),
            }
        }
        Ok(Some(rows))
    }
}
