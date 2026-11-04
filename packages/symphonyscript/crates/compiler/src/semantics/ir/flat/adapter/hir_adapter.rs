use super::super::file_rec::FileRec;
use super::super::flat_ir::FlatIr;
use super::super::top_rec::TopRec;
use super::super::{ABSENT, NONE, index};
use crate::semantics::Internal;
use crate::semantics::ir::{
    BlockId, ExprId, FnDecl, FnId, Hir, Ident, Input, InputId, LetDecl, LetId, Loc, Param,
    PatternDecl, PatternId, StateDecl, StateId, Top, UnknownNames,
};

pub(super) struct Adapter<'h> {
    pub hir: &'h mut Hir,
    pub ir: &'h FlatIr,
    pub file: u32,
    pub rec: FileRec,
    pub exprs_of: Vec<(u32, ExprId)>,
}

pub(crate) fn adapt_file(hir: &mut Hir, ir: &FlatIr, file: usize) -> Result<(), Internal> {
    adapt_file_mapped(hir, ir, file).map(|_| ())
}

pub(crate) fn adapt_file_mapped(
    hir: &mut Hir,
    ir: &FlatIr,
    file: usize,
) -> Result<Vec<(u32, ExprId)>, Internal> {
    let rec = *ir.files.get(file).ok_or(Internal::missed("IR file"))?;
    let index = u32::try_from(hir.files.len()).unwrap_or(u32::MAX);
    hir.files.push(ir.text(rec.path).to_string());
    let mut adapter = Adapter {
        hir,
        ir,
        file: index,
        rec,
        exprs_of: Vec::new(),
    };
    for at in rec.top.0..rec.top.1 {
        let top = *ir
            .top
            .get(at as usize)
            .ok_or(Internal::missed("IR top item"))?;
        adapter.top(top)?;
    }
    let skipped = adapter.region_names(rec.regions);
    adapter.hir.skipped_top.join(&skipped);
    Ok(adapter.exprs_of)
}

impl Adapter<'_> {
    pub fn loc(&self, lo: u32, hi: u32) -> Loc {
        Loc {
            file: self.file,
            start: lo,
            end: hi,
        }
    }

    pub fn ident(&self, name: u32) -> Option<Ident> {
        let entry = self.ir.name(name)?;
        if entry.faulty {
            return None;
        }
        Some(Ident {
            text: self.ir.symbols.text(entry.sym).to_string(),
            loc: self.loc(entry.lo, entry.hi),
        })
    }

    pub fn region_names(&self, regions: (u32, u32)) -> UnknownNames {
        let mut names = Vec::new();
        for at in regions.0..regions.1 {
            if let Some(region) = self.ir.regions.get(at as usize) {
                names.extend(self.ir.names_in(&self.rec, *region).map(str::to_string));
            }
        }
        UnknownNames::of(names)
    }

    pub fn emit_drawn(&mut self, drawn: (u32, u32)) -> Result<(), Internal> {
        for at in drawn.0..drawn.1 {
            let entry = *self
                .ir
                .drawn
                .get(at as usize)
                .ok_or(Internal::missed("IR diagnostic"))?;
            let loc = self.loc(entry.lo, entry.hi);
            let summary = self.ir.text(entry.summary).to_string();
            self.hir.drawn.push((entry.code, loc, summary));
        }
        Ok(())
    }

    fn placeholder(&mut self, name: u32) {
        if let Some(ident) = self.ident(name) {
            let id = index(self.hir.placeholders.len());
            self.hir.placeholders.push(ident);
            self.hir.top.push(Top::Placeholder(id));
        }
    }

    fn declared(&mut self, name: u32, faulty_line: bool) -> Option<Ident> {
        let ident = self.ident(name)?;
        if faulty_line {
            let id = index(self.hir.placeholders.len());
            self.hir.faulty_names.insert(ident.loc);
            self.hir.placeholders.push(ident.clone());
            self.hir.top.push(Top::Placeholder(id));
        }
        Some(ident)
    }

    fn top(&mut self, top: TopRec) -> Result<(), Internal> {
        match top {
            TopRec::Let(at) => self.top_let(at),
            TopRec::Cell(at) => self.cell(at),
            TopRec::Fn(at) => self.function(at),
            TopRec::Pattern(at) => self.pattern(at),
            TopRec::Placement(item) => {
                let mut pending = Vec::new();
                let id = self.placement(item, &mut pending)?;
                self.fill(pending)?;
                self.hir.top.push(Top::Placement(id));
                Ok(())
            }
            TopRec::Placeholder { name, drawn } => {
                self.emit_drawn(drawn)?;
                self.placeholder(name);
                Ok(())
            }
            TopRec::Drawn(drawn) => self.emit_drawn(drawn),
            TopRec::Skip => Ok(()),
        }
    }

    fn top_let(&mut self, at: u32) -> Result<(), Internal> {
        let rec = *self
            .ir
            .lets
            .get(at as usize)
            .ok_or(Internal::missed("IR let"))?;
        let binder = *self.ir.binder(rec.binder)?;
        let loc = self.loc(rec.lo, rec.hi);
        let value = self.expr(rec.value)?;
        let name = self.declared(binder.name, rec.faulty_line);
        let ty = self.annotation(binder.annotation)?;
        let id = LetId(index(self.hir.lets.len()));
        self.hir.lets.push(LetDecl {
            name,
            ty,
            value,
            loc,
        });
        self.hir.top.push(Top::Let(id));
        Ok(())
    }

    pub fn annotation(
        &mut self,
        root: u32,
    ) -> Result<Option<crate::semantics::ir::TypeRef>, Internal> {
        if root == NONE {
            return Ok(None);
        }
        self.type_ref(root).map(Some)
    }

    fn cell(&mut self, at: u32) -> Result<(), Internal> {
        let rec = *self
            .ir
            .cells
            .get(at as usize)
            .ok_or(Internal::missed("IR cell"))?;
        let binder = *self.ir.binder(rec.binder)?;
        let loc = self.loc(rec.lo, rec.hi);
        self.emit_drawn(rec.drawn)?;
        let name = self.declared(binder.name, rec.faulty_line);
        let ty = self.annotation(binder.annotation)?;
        if rec.input {
            let value = self.expr(rec.init)?;
            let id = InputId(index(self.hir.inputs.len()));
            self.hir.inputs.push(Input {
                name,
                word: rec.word,
                capacity: rec.capacity,
                ty,
                value,
                loc,
            });
            self.hir.top.push(Top::Input(id));
            return Ok(());
        }
        let init = if rec.init == NONE {
            None
        } else {
            Some(self.expr(rec.init)?)
        };
        let mut clock = Vec::new();
        for kid in rec.clock.0..rec.clock.1 {
            let entry = self.ir.kid(kid)?;
            clock.push(self.expr(entry)?);
        }
        let value = self.expr(rec.rule)?;
        let id = StateId(index(self.hir.states.len()));
        self.hir.states.push(StateDecl {
            name,
            word: rec.word,
            init,
            clock,
            ty,
            value,
            loc,
        });
        self.hir.top.push(Top::State(id));
        Ok(())
    }

    fn params(&mut self, params: (u32, u32), patterned: bool) -> Result<Vec<Param>, Internal> {
        let mut built = Vec::new();
        for at in params.0..params.1 {
            let binder = *self.ir.binder(at)?;
            let default = if binder.default == NONE {
                None
            } else {
                Some(self.expr(binder.default)?)
            };
            built.push(Param {
                name: self.ident(binder.name),
                default,
                is_default: patterned && binder.is_default,
                loc: self.loc(binder.lo, binder.hi),
            });
        }
        Ok(built)
    }

    pub fn local_lets(&mut self, lets: (u32, u32)) -> Result<Vec<LetDecl>, Internal> {
        let mut built = Vec::new();
        for at in lets.0..lets.1 {
            let rec = *self
                .ir
                .lets
                .get(at as usize)
                .ok_or(Internal::missed("IR let"))?;
            let binder = *self.ir.binder(rec.binder)?;
            let value = self.expr(rec.value)?;
            built.push(LetDecl {
                name: self.ident(binder.name),
                ty: None,
                value,
                loc: self.loc(rec.lo, rec.hi),
            });
        }
        Ok(built)
    }

    fn function(&mut self, at: u32) -> Result<(), Internal> {
        let rec = *self
            .ir
            .fns
            .get(at as usize)
            .ok_or(Internal::missed("IR function"))?;
        let loc = self.loc(rec.lo, rec.hi);
        let params = self.params(rec.params, false)?;
        let lets = self.local_lets(rec.lets)?;
        let value = self.expr(rec.value)?;
        let name = self.declared(rec.name, rec.faulty_line);
        let decl = FnDecl {
            name,
            params,
            skipped_params: self.region_names(rec.regions),
            faulty_head: rec.regions.1 > rec.regions.0,
            lets,
            value,
            loc,
        };
        let id = FnId(index(self.hir.fns.len()));
        self.hir.fns.push(decl);
        self.hir.top.push(Top::Fn(id));
        Ok(())
    }

    fn pattern(&mut self, at: u32) -> Result<(), Internal> {
        let rec = *self
            .ir
            .patterns
            .get(at as usize)
            .ok_or(Internal::missed("IR pattern"))?;
        let loc = self.loc(rec.lo, rec.hi);
        let params = self.params(rec.params, true)?;
        let (lets, body) = if rec.body == ABSENT {
            (Vec::new(), self.absent_block())
        } else {
            let block = *self
                .ir
                .blocks
                .get(rec.body as usize)
                .ok_or(Internal::missed("IR block"))?;
            let lets = self.local_lets(block.lets)?;
            let id = self.empty_block();
            self.fill(vec![(rec.body, id)])?;
            (lets, id)
        };
        let name = self.declared(rec.name, rec.faulty_line);
        let decl = PatternDecl {
            name,
            params,
            skipped_params: self.region_names(rec.regions),
            skipped_default: rec.skipped_default,
            lets,
            cells: Vec::new(),
            body,
            loc,
        };
        let id = PatternId(index(self.hir.patterns.len()));
        self.hir.patterns.push(decl);
        self.hir.top.push(Top::Pattern(id));
        Ok(())
    }

    pub fn absent_block(&mut self) -> BlockId {
        let id = BlockId(index(self.hir.blocks.len()));
        self.hir.blocks.push(crate::semantics::ir::Block {
            items: Vec::new(),
            unknown: UnknownNames {
                every: true,
                names: Vec::new(),
            },
        });
        id
    }

    pub fn empty_block(&mut self) -> BlockId {
        let id = BlockId(index(self.hir.blocks.len()));
        self.hir.blocks.push(crate::semantics::ir::Block::default());
        id
    }
}
