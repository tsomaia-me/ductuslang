use crate::semantics::ir::flat::{BlockRec, FlatIr, ForBlockRec, ItemRec};

pub(crate) fn bundle_fors(ir: &FlatIr) -> Option<(FlatIr, Vec<u32>)> {
    let fors: Vec<usize> = ir
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            matches!(item, ItemRec::Bundle(rec)
                if rec.each.is_some() && !rec.faulty && rec.members.0 == rec.members.1)
        })
        .map(|(at, _)| at)
        .collect();
    if fors.is_empty() {
        return None;
    }
    let mut owned = ir.clone();
    let mut loops = Vec::new();
    for at in fors {
        let Some(ItemRec::Bundle(rec)) = owned.items.get(at).copied() else {
            continue;
        };
        let each = rec.each?;
        let line = u32::try_from(owned.items.len()).ok()?;
        owned.items.push(ItemRec::Line(each.members));
        let start = u32::try_from(owned.block_items.len()).ok()?;
        owned.block_items.push(line);
        let members = u32::try_from(owned.members.len()).ok()?;
        let regions = u32::try_from(owned.regions.len()).ok()?;
        let lets = u32::try_from(owned.lets.len()).ok()?;
        let body = u32::try_from(owned.blocks.len()).ok()?;
        owned.blocks.push(BlockRec {
            items: (start, start + 1),
            members: (members, members),
            regions: (regions, regions),
            lets: (lets, lets),
        });
        *owned.items.get_mut(at)? = ItemRec::For(ForBlockRec {
            head: each.head,
            body,
            lo: rec.lo,
            hi: rec.hi,
        });
        loops.push(u32::try_from(at).ok()?);
    }
    Some((owned, loops))
}
