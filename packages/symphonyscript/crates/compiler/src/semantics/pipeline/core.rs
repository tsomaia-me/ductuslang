use std::collections::BTreeMap;

use symphonyscript_core::{Limits, Span, Vocabulary};

use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::instance_row::InstanceRow;
use super::store::Store;
use crate::diagnostics::{Code, Diagnostic, Related};
use crate::semantics::Internal;
use crate::semantics::collection::Variants;
use crate::semantics::ir::flat::FlatIr;

pub(crate) type Stage<T> = Result<Result<T, Declined>, Internal>;

pub(crate) struct Core<'a> {
    pub ir: &'a FlatIr,
    pub vocabulary: &'a Vocabulary,
    pub variants: &'a Variants,
    pub limits: Limits,
    pub diagnostics: Vec<Diagnostic>,
    pub placeholders: Vec<u32>,
    pub decl: Vec<DeclRef>,
    pub bind: Vec<DeclRef>,
    pub facts: Vec<Option<Fact>>,
    pub kept: Vec<bool>,
    pub let_state: Vec<u8>,
    pub let_file: Vec<u32>,
    pub cyclic: Vec<bool>,
    pub owned: Vec<bool>,
    pub reactive: Vec<bool>,
    pub cell_uses: Vec<[bool; 2]>,
    pub owning: bool,

    pub rule_runs: Vec<[bool; 7]>,
    pub reads_cell: Vec<Option<bool>>,
    pub instances: Vec<InstanceRow>,
    pub top_instances: Vec<u32>,
    pub store: Store,
    pub declined: bool,
    pub let_values: Vec<[Option<CoreValue>; 7]>,
    pub uses: BTreeMap<(u32, usize), ((bool, u32), CoreValue)>,
}

impl<'a> Core<'a> {
    pub fn new(
        ir: &'a FlatIr,
        vocabulary: &'a Vocabulary,
        variants: &'a Variants,
        limits: Limits,
    ) -> Self {
        let nodes = ir.op.len();
        Core {
            ir,
            vocabulary,
            variants,
            limits,
            diagnostics: Vec::new(),
            placeholders: Vec::new(),
            decl: Vec::new(),
            bind: vec![DeclRef::None; nodes],
            facts: vec![None; nodes],
            kept: vec![false; nodes],
            let_state: vec![0; ir.lets.len()],
            let_file: vec![0; ir.lets.len()],
            cyclic: vec![false; ir.lets.len()],
            owned: vec![false; ir.lets.len()],
            reactive: vec![false; nodes],
            cell_uses: vec![[false, false]; ir.cells.len()],
            owning: false,

            rule_runs: vec![[false; 7]; ir.cells.len()],
            reads_cell: vec![None; ir.lets.len()],
            instances: Vec::new(),
            top_instances: Vec::new(),
            store: Store::default(),
            declined: false,
            let_values: vec![[None, None, None, None, None, None, None]; ir.lets.len()],
            uses: BTreeMap::new(),
        }
    }

    pub fn local(&self, node: u32) -> Result<usize, Internal> {
        node.checked_sub(self.ir.base)
            .map(|local| local as usize)
            .filter(|local| *local < self.ir.op.len())
            .ok_or(Internal::missed("new-core node"))
    }

    pub fn file_of(&self, node: u32) -> u32 {
        let at = self.ir.files.partition_point(|file| file.nodes.1 <= node);
        u32::try_from(at).unwrap_or(0)
    }

    pub fn span(&self, file: u32, lo: u32, hi: u32) -> Span {
        let path = self
            .ir
            .files
            .get(file as usize)
            .map(|rec| self.ir.text(rec.path).to_string())
            .unwrap_or_default();
        Span {
            file: path,
            range: lo..hi,
        }
    }

    pub fn node_span(&self, node: u32) -> Result<Span, Internal> {
        let (lo, hi) = self.ir.span(node)?;
        Ok(self.span(self.file_of(node), lo, hi))
    }

    pub fn draw(&mut self, code: Code, span: Span, summary: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(code, span, summary));
    }

    pub fn draw_related(
        &mut self,
        code: Code,
        span: Span,
        related: Vec<(Span, String)>,
        summary: impl Into<String>,
    ) {
        let related = related
            .into_iter()
            .map(|(span, label)| Related { span, label })
            .collect();
        self.diagnostics
            .push(Diagnostic::new(code, span, summary).with_related(related));
    }

    pub fn draw_at(
        &mut self,
        code: Code,
        node: u32,
        summary: impl Into<String>,
    ) -> Result<(), Internal> {
        let span = self.node_span(node)?;
        self.draw(code, span, summary);
        Ok(())
    }

    pub fn kid(&self, at: u32) -> Result<u32, Internal> {
        self.ir.kid(at)
    }
}
