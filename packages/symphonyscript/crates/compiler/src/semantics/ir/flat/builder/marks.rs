use super::super::flat_ir::FlatIr;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Marks {
    nodes: usize,
    kids: usize,
    ints: usize,
    floats: usize,
    suffixes: usize,
    texts: usize,
    parts: usize,
    regions: usize,
    binders: usize,
    drawn: usize,
    cells: usize,
    lets: usize,
    fns: usize,
    patterns: usize,
    types: usize,
    fields: usize,
    fors: usize,
    items: usize,
    members: usize,
    arms: usize,
    attr_entries: usize,
    flags: usize,
    blocks: usize,
    block_items: usize,
    pub staged: usize,
    pub errs: usize,
    pub empty_type: bool,
}

impl Marks {
    pub fn of(ir: &FlatIr, staged: usize, errs: usize) -> Self {
        Self {
            nodes: ir.op.len(),
            kids: ir.kids.len(),
            ints: ir.ints.len(),
            floats: ir.floats.len(),
            suffixes: ir.suffixes.len(),
            texts: ir.texts.len(),
            parts: ir.parts.len(),
            regions: ir.regions.len(),
            binders: ir.binders.len(),
            drawn: ir.drawn.len(),
            cells: ir.cells.len(),
            lets: ir.lets.len(),
            fns: ir.fns.len(),
            patterns: ir.patterns.len(),
            types: ir.types.len(),
            fields: ir.fields.len(),
            fors: ir.fors.len(),
            items: ir.items.len(),
            members: ir.members.len(),
            arms: ir.arms.len(),
            attr_entries: ir.attr_entries.len(),
            flags: ir.flags.len(),
            blocks: ir.blocks.len(),
            block_items: ir.block_items.len(),
            staged,
            errs,
            empty_type: false,
        }
    }

    pub fn truncate(&self, ir: &mut FlatIr) {
        ir.op.truncate(self.nodes);
        ir.a.truncate(self.nodes);
        ir.b.truncate(self.nodes);
        ir.c.truncate(self.nodes);
        ir.lo.truncate(self.nodes);
        ir.hi.truncate(self.nodes);
        ir.kids.truncate(self.kids);
        ir.ints.truncate(self.ints);
        ir.floats.truncate(self.floats);
        ir.suffixes.truncate(self.suffixes);
        ir.texts.truncate(self.texts);
        ir.parts.truncate(self.parts);
        ir.regions.truncate(self.regions);
        ir.binders.truncate(self.binders);
        ir.drawn.truncate(self.drawn);
        ir.cells.truncate(self.cells);
        ir.lets.truncate(self.lets);
        ir.fns.truncate(self.fns);
        ir.patterns.truncate(self.patterns);
        ir.types.truncate(self.types);
        ir.fields.truncate(self.fields);
        ir.fors.truncate(self.fors);
        ir.items.truncate(self.items);
        ir.members.truncate(self.members);
        ir.arms.truncate(self.arms);
        ir.attr_entries.truncate(self.attr_entries);
        ir.flags.truncate(self.flags);
        ir.blocks.truncate(self.blocks);
        ir.block_items.truncate(self.block_items);
    }
}
