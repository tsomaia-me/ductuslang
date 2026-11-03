use super::core::{Core, Stage};
use super::declined::Declined;
use super::instance_row::InstanceRow;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ItemRec, NONE, PlacementRec, TopRec};

impl Core<'_> {
    pub fn placement(&self, item: u32) -> Result<PlacementRec, Internal> {
        match self.ir.item(item)? {
            ItemRec::Placement(placement) => Ok(*placement),
            ItemRec::Wire(_)
            | ItemRec::Bundle(_)
            | ItemRec::For(_)
            | ItemRec::When(_)
            | ItemRec::Line(_)
            | ItemRec::Paren(_) => Err(Internal::missed("IR placement")),
        }
    }

    fn row(&mut self, item: u32, file: u32, parent: u32) -> Stage<u32> {
        CountedWalk::CoreExpandRow.step();
        let placement = self.placement(item)?;
        let Some(kind_name) = self.ir.name_text(placement.kind) else {
            return Ok(Err(Declined));
        };
        let Some(kind) = self.vocabulary.node_kind_named(kind_name) else {
            return Ok(Err(Declined));
        };
        if parent != NONE && placement.as_name != NONE {
            return Ok(Err(Declined));
        }
        let row = u32::try_from(self.instances.len()).map_err(|_| Internal::missed("row"))?;
        self.instances.push(InstanceRow {
            kind,
            item,
            file,
            children: (0, 0),
        });
        Ok(Ok(row))
    }

    pub fn expand(&mut self) -> Stage<()> {
        let mut pending: Vec<u32> = Vec::new();
        for (file, rec) in self.ir.files.iter().enumerate() {
            let file = u32::try_from(file).unwrap_or(0);
            for at in rec.top.0..rec.top.1 {
                let top = *self
                    .ir
                    .top
                    .get(at as usize)
                    .ok_or(Internal::missed("IR top item"))?;
                if let TopRec::Placement(item) = top {
                    let row = match self.row(item, file, NONE)? {
                        Ok(row) => row,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    self.top_instances.push(row);
                    pending.push(row);
                }
            }
        }
        let mut next = 0;
        while let Some(&parent) = pending.get(next) {
            next += 1;
            let record = *self
                .instances
                .get(parent as usize)
                .ok_or(Internal::missed("row"))?;
            let placement = self.placement(record.item)?;
            if placement.body == NONE {
                continue;
            }
            let block = *self
                .ir
                .blocks
                .get(placement.body as usize)
                .ok_or(Internal::missed("IR block"))?;
            let first = u32::try_from(self.instances.len()).unwrap_or(u32::MAX);
            for at in block.items.0..block.items.1 {
                let item = *self
                    .ir
                    .block_items
                    .get(at as usize)
                    .ok_or(Internal::missed("IR block item"))?;
                let row = match self.row(item, record.file, parent)? {
                    Ok(row) => row,
                    Err(declined) => return Ok(Err(declined)),
                };
                pending.push(row);
            }
            let end = u32::try_from(self.instances.len()).unwrap_or(u32::MAX);
            if let Some(row) = self.instances.get_mut(parent as usize) {
                row.children = (first, end);
            }
        }
        Ok(Ok(()))
    }
}
