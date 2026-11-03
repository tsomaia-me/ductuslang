use super::attr_entry_rec::AttrEntryRec;
use super::binder::Binder;
use super::block_rec::BlockRec;
use super::cell_rec::CellRec;
use super::drawn::Drawn;
use super::field_rec::FieldRec;
use super::file_rec::FileRec;
use super::flag_rec::FlagRec;
use super::float_entry::FloatEntry;
use super::fn_rec::FnRec;
use super::for_rec::ForRec;
use super::item_rec::ItemRec;
use super::let_rec::LetRec;
use super::name_range::NameRange;
use super::op::Op;
use super::part::Part;
use super::pattern_rec::PatternRec;
use super::symbols::Symbols;
use super::top_rec::TopRec;
use super::type_rec::TypeRec;
use crate::semantics::Internal;
use crate::semantics::ir::Suffix;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct FlatIr {
    pub base: u32,
    pub op: Vec<Op>,
    pub a: Vec<u32>,
    pub b: Vec<u32>,
    pub c: Vec<u32>,
    pub lo: Vec<u32>,
    pub hi: Vec<u32>,
    pub kids: Vec<u32>,
    pub ints: Vec<i128>,
    pub floats: Vec<FloatEntry>,
    pub suffixes: Vec<Suffix>,
    pub texts: String,
    pub parts: Vec<Part>,
    pub names: Vec<NameRange>,
    pub name_order: Vec<u32>,
    pub regions: Vec<(u32, u32)>,
    pub binders: Vec<Binder>,
    pub drawn: Vec<Drawn>,
    pub cells: Vec<CellRec>,
    pub lets: Vec<LetRec>,
    pub fns: Vec<FnRec>,
    pub patterns: Vec<PatternRec>,
    pub types: Vec<TypeRec>,
    pub fields: Vec<FieldRec>,
    pub fors: Vec<ForRec>,
    pub items: Vec<ItemRec>,
    pub members: Vec<u32>,
    pub arms: Vec<(u32, u32)>,
    pub attr_entries: Vec<AttrEntryRec>,
    pub flags: Vec<FlagRec>,
    pub blocks: Vec<BlockRec>,
    pub block_items: Vec<u32>,
    pub top: Vec<TopRec>,
    pub files: Vec<FileRec>,
    pub file_first: Vec<u32>,
    pub symbols: Symbols,
}

impl FlatIr {
    pub fn after(prelude: &FlatIr) -> Self {
        Self {
            base: prelude.end(),
            symbols: prelude.symbols.layer(),
            ..Self::default()
        }
    }

    pub fn freeze(&mut self) {
        self.symbols.freeze();
    }

    pub fn end(&self) -> u32 {
        self.base
            .saturating_add(u32::try_from(self.op.len()).unwrap_or(u32::MAX))
    }

    fn local(&self, node: u32) -> Result<usize, Internal> {
        node.checked_sub(self.base)
            .map(|local| local as usize)
            .filter(|local| *local < self.op.len())
            .ok_or(Internal::missed("IR node"))
    }

    pub fn node(&self, node: u32) -> Result<(Op, u32, u32, u32), Internal> {
        let local = self.local(node)?;
        let missed = || Internal::missed("IR node");
        Ok((
            *self.op.get(local).ok_or_else(missed)?,
            *self.a.get(local).ok_or_else(missed)?,
            *self.b.get(local).ok_or_else(missed)?,
            *self.c.get(local).ok_or_else(missed)?,
        ))
    }

    pub fn span(&self, node: u32) -> Result<(u32, u32), Internal> {
        let local = self.local(node)?;
        let missed = || Internal::missed("IR node");
        Ok((
            *self.lo.get(local).ok_or_else(missed)?,
            *self.hi.get(local).ok_or_else(missed)?,
        ))
    }

    pub fn kid(&self, at: u32) -> Result<u32, Internal> {
        self.kids
            .get(at as usize)
            .copied()
            .ok_or(Internal::missed("IR operand list"))
    }

    pub fn name(&self, name: u32) -> Option<&NameRange> {
        self.names.get(name as usize)
    }

    pub fn name_text(&self, name: u32) -> Option<&str> {
        self.name(name).map(|entry| self.symbols.text(entry.sym))
    }

    pub fn text(&self, range: (u32, u32)) -> &str {
        self.texts
            .get(range.0 as usize..range.1 as usize)
            .unwrap_or_default()
    }

    pub fn item(&self, item: u32) -> Result<&ItemRec, Internal> {
        self.items
            .get(item as usize)
            .ok_or(Internal::missed("IR item"))
    }

    pub fn binder(&self, binder: u32) -> Result<&Binder, Internal> {
        self.binders
            .get(binder as usize)
            .ok_or(Internal::missed("IR binder"))
    }

    pub fn names_in(&self, file: &FileRec, region: (u32, u32)) -> impl Iterator<Item = &str> {
        let order = self
            .name_order
            .get(file.names.0 as usize..file.names.1 as usize)
            .unwrap_or_default();
        let lo_of = |index: &u32| self.names.get(*index as usize).map_or(0, |name| name.lo);
        let from = order.partition_point(|index| lo_of(index) < region.0);
        order
            .get(from..)
            .unwrap_or_default()
            .iter()
            .take_while(move |index| lo_of(index) < region.1)
            .filter_map(|index| self.name_text(*index))
    }
}
