use std::collections::{BTreeMap, HashMap, HashSet};

use symphonyscript_core::{Limits, Span, Vocabulary};

use super::attr_read::AttrRead;
use super::body_instance::BodyInstance;
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::history::history_targets;
use super::instance_row::InstanceRow;
use super::memo_entry::MemoEntry;
use super::memo_key::MemoKey;
use super::need::Need;
use super::recording::Recording;
use super::store::Store;
use super::wire_row::WireRow;
use crate::diagnostics::{Code, Diagnostic, Related};
use crate::semantics::Internal;
use crate::semantics::collection::Variants;
use crate::semantics::ir::flat::{FlatIr, NONE};
use crate::semantics::operators::EvalTy;

pub(crate) type Stage<T> = Result<Result<T, Declined>, Internal>;

pub(crate) type UseKey = (u32, usize, Vec<u32>, Vec<u32>);

pub(crate) type BodyKey = (u32, u32, Vec<Fact>, Vec<bool>);

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
    pub own_calls: Vec<(u32, u32, Vec<CoreValue>)>,
    pub top_drawn: Vec<Diagnostic>,
    pub typing_bodies: u32,
    pub unchecked: bool,
    pub fitting: bool,
    pub unflowed: HashSet<u32>,
    pub unused_reads: HashSet<(u32, u32)>,
    pub driving: bool,
    pub need: Option<Need>,

    pub rule_reads: Vec<[Option<(EvalTy, bool)>; 7]>,
    pub cell_order: Vec<u32>,
    pub cell_cyclic: Vec<bool>,
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
    pub outer_owner: HashMap<u32, u32>,
    pub call_memo: HashMap<MemoKey, MemoEntry>,
    pub recordings: Vec<Recording>,
    pub reaches_out: Vec<bool>,
    pub wires: Vec<WireRow>,
    pub candidates: Vec<u32>,
    pub row_of_item: Vec<u32>,
    pub expanded: u64,
    pub reference_roots: Vec<u32>,
    pub path_rows: HashMap<u32, u32>,
    pub paths: Vec<u32>,
    pub attr_reads: Vec<AttrRead>,
    pub attr_index: HashMap<(u32, u32), u32>,
    pub attr_values: Vec<[Option<CoreValue>; 7]>,
    pub attr_order: Vec<u32>,
    pub field_receivers: Vec<u32>,
    pub child_index: Vec<(u32, u32, u32)>,
    pub to_wires: Vec<(u32, u32)>,
    pub body_path_faults: HashMap<u32, (Vec<Diagnostic>, bool)>,
    pub deferred_to: Vec<(u32, u32)>,
    pub to_choices: HashMap<u32, EvalTy>,
    pub wire_now: u32,
    pub feed_env: u32,
    pub row_reads: BTreeMap<(u32, u32), u32>,
    pub row_choices: HashMap<(u32, u32), EvalTy>,
    pub ref_insts: HashMap<(u32, u32, u32), u32>,
    pub typed_as: HashMap<u32, u32>,
    pub pattern_files: Vec<u32>,
    pub patterns_used: HashSet<u32>,
    pub cyclic_patterns: Vec<bool>,
    pub named_patterns: Vec<bool>,
    pub use_links: Vec<(u32, u32)>,
    pub use_spans: HashMap<u32, Span>,
    pub use_args: HashMap<u32, Vec<(u32, u32)>>,
    pub use_values: HashMap<(u32, u32, usize), CoreValue>,
    pub use_faults: HashMap<u32, Vec<Diagnostic>>,
    pub default_faults: HashMap<(u32, u32), Vec<Diagnostic>>,
    pub typing_now: u32,
    pub inst_row: HashMap<u32, (u32, u32)>,
    pub owner_insts: HashMap<u32, Vec<u32>>,
    pub inst_reads: BTreeMap<(u32, u32), u32>,
    pub path_owners: Option<std::collections::HashSet<u32>>,
    pub per_inst_paths: std::collections::HashSet<u32>,
    pub child_scopes: Vec<(u32, Vec<(u32, u32)>)>,
    pub scope_of: HashMap<u32, u32>,
    pub loops: Vec<u32>,
    pub envs: Vec<super::env::Env>,
    pub env_vals: Vec<CoreValue>,
    pub loop_items: HashMap<(u32, u32, usize), CoreValue>,
    pub open_children: HashMap<u32, Vec<symphonyscript_core::NodeKind>>,
    pub loop_reads: BTreeMap<(u32, u32, usize), CoreValue>,
    pub placeholder_uses: Vec<(u32, u32, Span, u32)>,
    pub default_nodes: HashMap<u32, u32>,
    pub default_insts: HashMap<(u32, u32), u32>,
    pub deferred_uses: Vec<(u32, Vec<u32>)>,
    pub deferred_params: HashSet<(u32, u32)>,
    pub pattern_items: HashMap<u32, u32>,
    pub own_reads: HashSet<u32>,
    pub env_rows: HashMap<(u32, u32), u32>,
    pub env_reads: BTreeMap<(u32, u32), u32>,
    pub use_names: HashMap<(u32, u32), u32>,
    pub use_index: Vec<(u32, u32, u32)>,
    pub group_items: HashSet<u32>,
    pub path_env: u32,
    pub env_path_rows: HashMap<(u32, u32), u32>,
    pub env_param_reads: HashMap<u32, Vec<(u32, u32)>>,
    pub sig_index: HashMap<(super::core::BodyKey, Vec<(u32, super::ref_bodies::Binding)>), u32>,
    pub history_target: Vec<u8>,
    pub cell_reads: Vec<super::groups::CellReads>,
    pub cell_group: Vec<u32>,
    pub groups: Vec<Vec<u32>>,
    pub group_reads: HashMap<u32, (u32, u32)>,
    pub group_kinds: HashMap<u32, Fact>,
    pub group_variable: HashSet<u32>,
    pub group_history: HashMap<u32, Vec<u32>>,
    pub group_open: HashSet<u32>,
    pub read_log: Vec<u32>,
    pub init_reads: Vec<super::init_read::InitRead>,
    pub init_open: super::init_read::Index,
    pub init_args: super::init_read::Index,
    pub init_serial: u64,
    pub cell_capacities: HashMap<u32, i128>,
    pub extent_cache: HashMap<u32, super::extent::Extent>,
    pub bare: HashSet<u32>,
    pub unknown_variants: HashSet<u32>,
    pub family_faults: HashSet<u32>,
    pub units_seen: bool,
    pub enums_seen: bool,
    pub bare_enums: HashMap<u32, symphonyscript_core::EnumId>,
    pub untyped_bare: HashSet<u32>,
    pub takers: Vec<(u32, symphonyscript_core::FamilyId, u32)>,
    pub unit_feeds: Vec<(u32, symphonyscript_core::FamilyId, usize, u32)>,
    pub taker_set: HashSet<(u32, symphonyscript_core::FamilyId, u32)>,
    pub unit_reaches: HashMap<
        (u32, u32, symphonyscript_core::FamilyId),
        std::rc::Rc<super::unit_reach::UnitReach>,
    >,
    pub unit_let_values: HashMap<(u32, usize, symphonyscript_core::FamilyId, usize), CoreValue>,
    pub unit_conflicted: HashSet<(u32, u32, u32)>,
    pub cycle_through_refused: bool,
    pub unbound_nodes: std::collections::HashSet<u32>,
    pub guards: Vec<(u32, u32)>,
    pub unit_conflicts: Vec<super::unit_conflict::UnitConflict>,
    pub default_scale: Option<(
        u32,
        symphonyscript_core::FamilyId,
        usize,
        crate::semantics::operators::EvalTy,
    )>,
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
            own_calls: Vec::new(),
            top_drawn: Vec::new(),
            typing_bodies: 0,
            unchecked: false,
            fitting: false,
            unflowed: HashSet::new(),
            unused_reads: HashSet::new(),
            driving: false,
            need: None,

            rule_reads: vec![[None; 7]; ir.cells.len()],
            cell_order: Vec::new(),
            cell_cyclic: vec![false; ir.cells.len()],
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
            outer_owner: HashMap::new(),
            call_memo: HashMap::new(),
            recordings: Vec::new(),
            reaches_out: vec![false; ir.fors.len()],
            wires: Vec::new(),
            candidates: Vec::new(),
            row_of_item: vec![NONE; ir.items.len()],
            expanded: 0,
            reference_roots: Vec::new(),
            path_rows: HashMap::new(),
            paths: vec![NONE; nodes],
            attr_reads: Vec::new(),
            attr_index: HashMap::new(),
            attr_values: Vec::new(),
            attr_order: Vec::new(),
            field_receivers: Vec::new(),
            child_index: Vec::new(),
            to_wires: Vec::new(),
            body_path_faults: HashMap::new(),
            deferred_to: Vec::new(),
            to_choices: HashMap::new(),
            wire_now: NONE,
            feed_env: NONE,
            row_reads: BTreeMap::new(),
            row_choices: HashMap::new(),
            ref_insts: HashMap::new(),
            typed_as: HashMap::new(),
            pattern_files: vec![0; ir.patterns.len()],
            cyclic_patterns: vec![false; ir.patterns.len()],
            named_patterns: vec![true; ir.patterns.len()],
            patterns_used: HashSet::new(),
            use_links: Vec::new(),
            use_spans: HashMap::new(),
            use_args: HashMap::new(),
            use_values: HashMap::new(),
            use_faults: HashMap::new(),
            default_faults: HashMap::new(),
            typing_now: 0,
            inst_row: HashMap::new(),
            owner_insts: HashMap::new(),
            inst_reads: BTreeMap::new(),
            path_owners: None,
            per_inst_paths: std::collections::HashSet::new(),
            child_scopes: Vec::new(),
            scope_of: HashMap::new(),
            loops: super::loops::loop_items(ir),
            envs: Vec::new(),
            env_vals: Vec::new(),
            loop_items: HashMap::new(),
            open_children: HashMap::new(),
            loop_reads: BTreeMap::new(),
            placeholder_uses: Vec::new(),
            default_nodes: HashMap::new(),
            default_insts: HashMap::new(),
            deferred_uses: Vec::new(),
            deferred_params: HashSet::new(),
            pattern_items: HashMap::new(),
            own_reads: HashSet::new(),
            env_rows: HashMap::new(),
            env_reads: BTreeMap::new(),
            use_names: HashMap::new(),
            use_index: Vec::new(),
            group_items: HashSet::new(),
            path_env: NONE,
            env_path_rows: HashMap::new(),
            env_param_reads: HashMap::new(),
            sig_index: HashMap::new(),
            history_target: history_targets(ir),
            cell_reads: Vec::new(),
            cell_group: Vec::new(),
            groups: Vec::new(),
            group_reads: HashMap::new(),
            group_kinds: HashMap::new(),
            group_variable: HashSet::new(),
            group_history: HashMap::new(),
            group_open: HashSet::new(),
            read_log: Vec::new(),
            init_reads: Vec::new(),
            init_open: HashMap::new(),
            init_args: HashMap::new(),
            init_serial: 0,
            cell_capacities: HashMap::new(),
            extent_cache: HashMap::new(),
            bare: HashSet::new(),
            unknown_variants: HashSet::new(),
            family_faults: HashSet::new(),
            units_seen: false,
            enums_seen: false,
            bare_enums: HashMap::new(),
            untyped_bare: HashSet::new(),
            takers: Vec::new(),
            unit_feeds: Vec::new(),
            taker_set: HashSet::new(),
            unit_reaches: HashMap::new(),
            unit_let_values: HashMap::new(),
            unit_conflicted: HashSet::new(),
            cycle_through_refused: false,
            unbound_nodes: std::collections::HashSet::new(),
            guards: Vec::new(),
            unit_conflicts: Vec::new(),
            default_scale: None,
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
        let diagnostic = self.call_chained(diagnostic);
        if self.use_links.is_empty() {
            return diagnostic;
        }
        let mut related = diagnostic.related.clone();
        for (item, _) in &self.use_links {
            if let Some(span) = self.use_spans.get(item) {
                related.push(Related {
                    span: span.clone(),
                    label: "in this use of the pattern".to_string(),
                });
            }
        }
        diagnostic.with_related(related)
    }

    pub fn call_chained(&self, diagnostic: Diagnostic) -> Diagnostic {
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

    pub fn draw_helped_at(
        &mut self,
        code: Code,
        node: u32,
        summary: impl Into<String>,
        help: impl Into<String>,
    ) -> Result<(), Internal> {
        let span = self.node_span(node)?;
        let diagnostic = Diagnostic::new(code, span, summary).with_help(help);
        let diagnostic = self.chained(diagnostic);
        self.push_drawn(diagnostic);
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
        let inst = self.owner_inst(owner);
        Ok(Some((
            self.typed_as.get(&inst).copied().unwrap_or(inst),
            node,
        )))
    }

    pub fn path_inst(&self, node: u32) -> Option<u32> {
        let owner = self
            .body_of
            .get(self.local(node).ok()?)
            .copied()
            .unwrap_or(NONE);
        if owner == NONE || self.inst == NONE {
            return None;
        }
        Some(self.owner_inst(owner))
    }

    pub fn owner_inst(&self, owner: u32) -> u32 {
        let mut at = self.inst;
        while let Some(body) = self.bodies.get(at as usize) {
            if body.function == owner {
                return at;
            }
            at = body.parent;
        }
        self.inst
    }

    pub fn inst_key(&self, node: u32) -> Result<u32, Internal> {
        Ok(match self.in_body(node)? {
            Some((inst, _)) => inst,
            None => self.inst,
        })
    }

    pub fn binding_owner(&self, node: u32) -> Result<u32, Internal> {
        Ok(match self.outer_owner.get(&node) {
            Some(owner) => *owner,
            None => self.body_of.get(self.local(node)?).copied().unwrap_or(NONE),
        })
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
    pub fn push_drawn(&mut self, diagnostic: Diagnostic) {
        self.record(&diagnostic);
        if self.typing_now > 0
            && let Some((_, inst)) = self.use_links.first().copied()
        {
            let mut own = diagnostic.clone();
            let keep = own.related.len().saturating_sub(self.use_links.len());
            own.related.truncate(keep);
            let inst = self.typed_as.get(&inst).copied().unwrap_or(inst);
            self.use_faults.entry(inst).or_default().push(own);
        }
        if self.eval_chain.is_empty() && self.use_links.len() < 2 {
            self.diagnostics.push(diagnostic);
        } else {
            self.push_one_per_chain(diagnostic, false);
        }
    }
}
