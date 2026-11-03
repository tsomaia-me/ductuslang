use std::collections::{BTreeMap, HashMap, HashSet};

use symphonyscript_core::{Limits, Span, Vocabulary};

use super::body_instance::BodyInstance;
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::instance_row::InstanceRow;
use super::store::Store;
use crate::diagnostics::{Code, Diagnostic, Related};
use crate::semantics::Internal;
use crate::semantics::collection::Variants;
use crate::semantics::ir::flat::{FlatIr, NONE};

pub(crate) type Stage<T> = Result<Result<T, Declined>, Internal>;

pub(crate) type UseKey = (u32, usize, Vec<u32>);

pub(crate) type BodyKey = (u32, Vec<Fact>, Vec<bool>);

pub(crate) type UseValue = ((bool, u32), CoreValue);

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
    pub unchecked: bool,

    pub rule_runs: Vec<[bool; 7]>,
    pub inst: u32,
    pub body_of: Vec<u32>,
    pub bodies: Vec<BodyInstance>,
    pub body_facts: HashMap<(u32, u32), Fact>,
    pub body_kept: HashSet<(u32, u32)>,
    pub body_reactive: HashSet<(u32, u32)>,
    pub call_body: HashMap<(u32, u32), u32>,
    pub eval_chain: Vec<u32>,
    pub typing_locals: HashSet<(u32, u32)>,
    pub reads_cell: Vec<Option<bool>>,
    pub instances: Vec<InstanceRow>,
    pub top_instances: Vec<u32>,
    pub store: Store,
    pub let_values: Vec<[Option<CoreValue>; 7]>,
    pub uses: BTreeMap<UseKey, UseValue>,
    pub fold_steps: u64,
    pub stop: Option<(usize, Diagnostic)>,
    pub fold_nodes: Vec<u32>,
    pub warned: HashSet<u32>,
    pub unknown_kind: bool,
    pub unknown_top: bool,
    pub unknown_wires: bool,
    pub unknown_children: HashSet<u32>,
    pub skipped_top: Vec<u32>,
    pub reached: Vec<bool>,
    pub body_index: HashMap<BodyKey, u32>,
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
            unchecked: false,

            rule_runs: vec![[false; 7]; ir.cells.len()],
            inst: NONE,
            body_of: vec![NONE; nodes],
            bodies: Vec::new(),
            body_facts: HashMap::new(),
            body_kept: HashSet::new(),
            body_reactive: HashSet::new(),
            call_body: HashMap::new(),
            eval_chain: Vec::new(),
            typing_locals: HashSet::new(),
            reads_cell: vec![None; ir.lets.len()],
            instances: Vec::new(),
            top_instances: Vec::new(),
            store: Store::default(),
            let_values: vec![[None, None, None, None, None, None, None]; ir.lets.len()],
            uses: BTreeMap::new(),
            fold_steps: 0,
            stop: None,
            fold_nodes: vec![NONE; ir.fors.len()],
            warned: HashSet::new(),
            unknown_kind: false,
            unknown_top: false,
            unknown_wires: false,
            unknown_children: HashSet::new(),
            skipped_top: Vec::new(),
            reached: vec![false; nodes],
            body_index: HashMap::new(),
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
        let diagnostic = Diagnostic::new(code, span, summary);
        let diagnostic = self.chained(diagnostic);
        self.push_drawn(diagnostic);
    }

    pub fn chained(&self, diagnostic: Diagnostic) -> Diagnostic {
        if self.eval_chain.is_empty() {
            return diagnostic;
        }
        let mut related = diagnostic.related.clone();
        for call in self.eval_chain.iter().rev() {
            if let Ok(span) = self.node_span(*call) {
                related.push(Related {
                    span,
                    label: "in this call".to_string(),
                });
            }
        }
        diagnostic.with_related(related)
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
        let diagnostic = Diagnostic::new(code, span, summary).with_related(related);
        let diagnostic = self.chained(diagnostic);
        self.push_drawn(diagnostic);
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

impl Core<'_> {
    pub fn in_body(&self, node: u32) -> Result<Option<(u32, u32)>, Internal> {
        let local = self.local(node)?;
        let owner = self.body_of.get(local).copied().unwrap_or(NONE);
        if owner == NONE {
            return Ok(None);
        }
        if self.inst == NONE {
            return Err(Internal::missed("a body instance"));
        }
        Ok(Some((self.inst, node)))
    }

    pub fn fact(&self, node: u32) -> Result<Fact, Internal> {
        let found = match self.in_body(node)? {
            Some(key) => self.body_facts.get(&key).cloned(),
            None => self.facts.get(self.local(node)?).cloned().flatten(),
        };
        found.ok_or(Internal::missed("new-core fact"))
    }

    pub fn has_fact(&self, node: u32) -> Result<bool, Internal> {
        Ok(match self.in_body(node)? {
            Some(key) => self.body_facts.contains_key(&key),
            None => self
                .facts
                .get(self.local(node)?)
                .is_some_and(Option::is_some),
        })
    }

    pub fn set_fact(&mut self, node: u32, fact: Fact, reactive: bool) -> Result<(), Internal> {
        match self.in_body(node)? {
            Some(key) => {
                self.body_facts.insert(key, fact);
                if reactive {
                    self.body_reactive.insert(key);
                }
            }
            None => {
                let local = self.local(node)?;
                if let Some(slot) = self.facts.get_mut(local) {
                    *slot = Some(fact);
                }
                if let Some(slot) = self.reactive.get_mut(local) {
                    *slot = reactive;
                }
            }
        }
        Ok(())
    }

    pub fn keep(&mut self, node: u32, fact: Fact) -> Result<Fact, Internal> {
        match self.in_body(node)? {
            Some(key) => {
                self.body_kept.insert(key);
            }
            None => {
                let local = self.local(node)?;
                if let Some(slot) = self.kept.get_mut(local) {
                    *slot = true;
                }
            }
        }
        Ok(fact)
    }

    pub fn is_kept(&self, node: u32) -> Result<bool, Internal> {
        Ok(match self.in_body(node)? {
            Some(key) => self.body_kept.contains(&key),
            None => self.kept.get(self.local(node)?).copied().unwrap_or(false),
        })
    }

    pub fn is_reactive(&self, node: u32) -> Result<bool, Internal> {
        Ok(match self.in_body(node)? {
            Some(key) => self.body_reactive.contains(&key),
            None => self
                .reactive
                .get(self.local(node)?)
                .copied()
                .unwrap_or(false),
        })
    }
}

impl Core<'_> {
    fn push_drawn(&mut self, diagnostic: Diagnostic) {
        if self.eval_chain.is_empty() {
            self.diagnostics.push(diagnostic);
        } else {
            self.push_one_per_chain(diagnostic, false);
        }
    }
}
