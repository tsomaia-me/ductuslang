use super::super::flat_ir::FlatIr;
use super::super::index;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Marks {
    nodes: u32,
    kids: u32,
    ints: u32,
    floats: u32,
    suffixes: u32,
    texts: u32,
    parts: u32,
    regions: u32,
    binders: u32,
    drawn: u32,
    cells: u32,
    lets: u32,
    fns: u32,
    patterns: u32,
    types: u32,
    fields: u32,
    fors: u32,
    items: u32,
    members: u32,
    arms: u32,
    attr_entries: u32,
    flags: u32,
    blocks: u32,
    block_items: u32,
    pub staged: u32,
    pub errs: u32,
    pub empty_type: bool,
}

impl Marks {
    pub fn of(ir: &FlatIr, staged: usize, errs: usize) -> Self {
        Self {
            nodes: index(ir.op.len()),
            kids: index(ir.kids.len()),
            ints: index(ir.ints.len()),
            floats: index(ir.floats.len()),
            suffixes: index(ir.suffixes.len()),
            texts: index(ir.texts.len()),
            parts: index(ir.parts.len()),
            regions: index(ir.regions.len()),
            binders: index(ir.binders.len()),
            drawn: index(ir.drawn.len()),
            cells: index(ir.cells.len()),
            lets: index(ir.lets.len()),
            fns: index(ir.fns.len()),
            patterns: index(ir.patterns.len()),
            types: index(ir.types.len()),
            fields: index(ir.fields.len()),
            fors: index(ir.fors.len()),
            items: index(ir.items.len()),
            members: index(ir.members.len()),
            arms: index(ir.arms.len()),
            attr_entries: index(ir.attr_entries.len()),
            flags: index(ir.flags.len()),
            blocks: index(ir.blocks.len()),
            block_items: index(ir.block_items.len()),
            staged: index(staged),
            errs: index(errs),
            empty_type: false,
        }
    }

    pub fn truncate(&self, ir: &mut FlatIr) {
        ir.op.truncate(self.nodes as usize);
        ir.a.truncate(self.nodes as usize);
        ir.b.truncate(self.nodes as usize);
        ir.c.truncate(self.nodes as usize);
        ir.lo.truncate(self.nodes as usize);
        ir.hi.truncate(self.nodes as usize);
        ir.kids.truncate(self.kids as usize);
        ir.ints.truncate(self.ints as usize);
        ir.floats.truncate(self.floats as usize);
        ir.suffixes.truncate(self.suffixes as usize);
        ir.texts.truncate(self.texts as usize);
        ir.parts.truncate(self.parts as usize);
        ir.regions.truncate(self.regions as usize);
        ir.binders.truncate(self.binders as usize);
        ir.drawn.truncate(self.drawn as usize);
        ir.cells.truncate(self.cells as usize);
        ir.lets.truncate(self.lets as usize);
        ir.fns.truncate(self.fns as usize);
        ir.patterns.truncate(self.patterns as usize);
        ir.types.truncate(self.types as usize);
        ir.fields.truncate(self.fields as usize);
        ir.fors.truncate(self.fors as usize);
        ir.items.truncate(self.items as usize);
        ir.members.truncate(self.members as usize);
        ir.arms.truncate(self.arms as usize);
        ir.attr_entries.truncate(self.attr_entries as usize);
        ir.flags.truncate(self.flags as usize);
        ir.blocks.truncate(self.blocks as usize);
        ir.block_items.truncate(self.block_items as usize);
    }
}
