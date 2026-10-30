use std::collections::BTreeMap;

use crate::semantics::Internal;
use crate::semantics::analysis::index::index;
use crate::semantics::ir::{AttrValue, ExprId, FnId, Hir, LetId, PlacementId};

type Roots<'h> = (Vec<ExprId>, BTreeMap<&'h str, Option<ExprId>>, bool);

pub(crate) struct Owners {
    lets: usize,
    fns: usize,
    attribute_starts: Vec<usize>,
    attributes: usize,
}

impl Owners {
    pub fn of(hir: &Hir) -> Owners {
        let mut attribute_starts = Vec::with_capacity(hir.placements.len());
        let mut attributes = 0usize;
        for placement in &hir.placements {
            attribute_starts.push(attributes);
            attributes = attributes.saturating_add(placement.attrs.len());
        }
        Owners {
            lets: hir.lets.len(),
            fns: hir.fns.len(),
            attribute_starts,
            attributes,
        }
    }

    pub fn count(&self) -> usize {
        self.lets + self.fns + self.attributes
    }

    pub fn function(&self, id: FnId) -> usize {
        self.lets + id.index()
    }

    pub fn attribute(&self, placement: PlacementId, position: usize) -> Result<usize, Internal> {
        let start = self
            .attribute_starts
            .get(placement.index())
            .ok_or(Internal::missed("placement"))?;
        Ok(self.lets + self.fns + start + position)
    }

    pub fn roots<'h>(&self, hir: &'h Hir, owner: usize) -> Result<Option<Roots<'h>>, Internal> {
        if owner < self.lets {
            let id = LetId(index(owner));
            if hir.is_prelude_let(id) {
                return Ok(None);
            }
            return Ok(Some((
                vec![hir.let_decl(id)?.value],
                BTreeMap::new(),
                false,
            )));
        }
        if owner < self.lets + self.fns {
            let function = FnId(index(owner - self.lets));
            if hir.is_prelude_fn(function) {
                return Ok(None);
            }
            let decl = hir.fn_decl(function)?;
            let mut locals: BTreeMap<&str, Option<ExprId>> = BTreeMap::new();
            for param in &decl.params {
                if let Some(name) = &param.name {
                    locals.insert(name.text.as_str(), None);
                }
            }
            for local in &decl.lets {
                if let Some(name) = &local.name {
                    locals.insert(name.text.as_str(), Some(local.value));
                }
            }
            let mut roots: Vec<ExprId> = decl
                .params
                .iter()
                .filter_map(|param| param.default)
                .collect();
            roots.extend(decl.lets.iter().map(|local| local.value));
            roots.push(decl.value);
            return Ok(Some((roots, locals, true)));
        }
        let attribute = owner - self.lets - self.fns;
        let placement = self
            .attribute_starts
            .partition_point(|start| *start <= attribute)
            .checked_sub(1)
            .ok_or(Internal::missed("attribute owner"))?;
        let start = self
            .attribute_starts
            .get(placement)
            .ok_or(Internal::missed("attribute owner"))?;
        let record = hir.placement(PlacementId(index(placement)))?;
        Ok(
            match record.attrs.get(attribute - start).map(|attr| attr.value) {
                Some(AttrValue::Expr(value)) => Some((vec![value], BTreeMap::new(), false)),
                Some(AttrValue::Set(_)) | None => None,
            },
        )
    }
}
