use std::collections::{BTreeMap, HashMap, HashSet};

use symphonyscript_core::{Limits, Span, Vocabulary};

use super::attr_read::AttrRead;
use super::body_instance::BodyInstance;
use super::call_chain::{CallChain, ChainArena};
use super::column::{Column, ColumnSet, PairColumn, PairSet, TripleColumn, TripleSet};
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
use crate::semantics::ir::flat::{FlatIr, NONE, WireRec};
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
    pub unflowed: ColumnSet,
    pub unused_reads: PairSet,
    pub unexpected_reads: PairSet,
    pub compared_reads: ColumnSet,
    pub placeholder_kinds: PairColumn<Fact>,
    pub not_values: Column<String>,
    pub not_values_drawn: ColumnSet,
    pub not_callables_drawn: ColumnSet,
    pub cyclic_let_reads: Column<u32>,
    pub let_seeds: Column<Fact>,
    pub pi: ColumnSet,
    pub compared_fits: Vec<(u32, u32)>,
    pub driving: bool,
    pub need: Option<Need>,

    pub rule_reads: Vec<[Option<(EvalTy, bool)>; 7]>,
    pub cell_order: Vec<u32>,
    pub cell_cyclic: Vec<bool>,
    pub inst: u32,
    pub body_of: Vec<u32>,
    pub bodies: Vec<BodyInstance>,
    pub body_facts: PairColumn<Fact>,
    pub body_kept: PairSet,
    pub body_reactive: PairSet,
    pub call_body: PairColumn<u32>,
    pub call_widths: PairColumn<EvalTy>,
    pub eval_chain: CallChain,
    pub chain_arena: ChainArena,
    pub typing_locals: PairSet,
    pub reads_cell: Vec<Option<bool>>,
    pub instances: Vec<InstanceRow>,
    pub top_instances: Vec<u32>,
    pub store: Store,
    pub let_values: Vec<[Option<CoreValue>; 7]>,
    pub uses: BTreeMap<UseKey, UseValue>,
    pub fold_steps: u64,
    pub stop: Option<(usize, Diagnostic)>,
    pub fold_nodes: Vec<u32>,
    pub warned: ColumnSet,
    pub unknown_kind: bool,
    pub unknown_top: bool,
    pub unknown_wires: bool,
    pub unknown_children: ColumnSet,
    pub skipped_top: Vec<u32>,
    pub reached: Vec<bool>,
    pub body_index: HashMap<BodyKey, u32>,
    pub value_index: HashMap<(BodyKey, Vec<Option<super::memo_arg::MemoArg>>), u32>,
    pub keyed_params: Option<Vec<Vec<bool>>>,
    pub outer_owner: Column<u32>,
    pub call_memo: HashMap<MemoKey, MemoEntry>,
    pub recordings: Vec<Recording>,
    pub reaches_out: Vec<bool>,
    pub wires: Vec<WireRow>,
    pub candidates: Vec<u32>,
    pub row_of_item: Vec<u32>,
    pub expanded: u64,
    pub reference_roots: Vec<u32>,
    pub path_rows: Column<u32>,
    pub paths: Vec<u32>,
    pub attr_reads: Vec<AttrRead>,
    pub attr_index: PairColumn<u32>,
    pub attr_values: Vec<[Option<CoreValue>; 7]>,
    pub attr_order: Vec<u32>,
    pub field_receivers: Vec<u32>,
    pub child_index: Vec<(u32, u32, u32)>,
    pub wire_index: Vec<(u32, u32, u32)>,
    pub wire_attr_index: PairColumn<u32>,
    pub to_wires: Vec<(u32, u32)>,
    pub body_path_faults: Column<(Vec<Diagnostic>, bool)>,
    pub deferred_to: Vec<(u32, u32)>,
    pub untyped_reads: Vec<u32>,
    pub open_reads: TripleSet,
    pub open_values: TripleSet,
    pub unrun_to_wires: Vec<(WireRec, u32)>,
    pub to_choices: Column<EvalTy>,
    pub wire_now: u32,
    pub wire_destinations: ColumnSet,
    pub choosing: u32,
    pub feed_env: u32,
    pub row_reads: PairColumn<u32>,
    pub choice_reads: PairSet,
    pub runtime_to: ColumnSet,
    pub row_choices: PairColumn<EvalTy>,
    pub ref_insts: TripleColumn<u32>,
    pub typed_as: Column<u32>,
    pub pattern_files: Vec<u32>,
    pub patterns_used: ColumnSet,
    pub cyclic_patterns: Vec<bool>,
    pub named_patterns: Vec<bool>,
    pub use_links: Vec<(u32, u32)>,
    pub use_spans: Column<Span>,
    pub use_args: Column<Vec<(u32, u32)>>,
    pub use_values: TripleColumn<CoreValue>,
    pub compile_time_folds: Vec<(u32, u32)>,
    pub use_faults: Column<Vec<Diagnostic>>,
    pub default_faults: PairColumn<Vec<Diagnostic>>,
    pub typing_now: u32,
    pub inst_row: Column<(u32, u32)>,
    pub owner_insts: Column<Vec<u32>>,
    pub inst_reads: PairColumn<u32>,
    pub path_owners: Option<ColumnSet>,
    pub per_inst_paths: ColumnSet,
    pub param_paths: ColumnSet,
    pub param_reads: Vec<(u32, u32)>,
    pub member_keys: PairSet,
    pub array_members: Column<i128>,
    pub expr_members: ColumnSet,
    pub cell_sizes: Column<u32>,
    pub runtime_members: ColumnSet,
    pub capacity_members: ColumnSet,
    pub history_capacities: Column<(u32, i128)>,
    pub dot_calls: ColumnSet,
    pub bundle_loops: ColumnSet,
    pub unknown_methods: ColumnSet,
    pub function_values: ColumnSet,
    pub path_free: Column<bool>,
    pub child_scopes: Vec<(u32, Vec<(u32, u32)>)>,
    pub scope_of: Column<u32>,
    pub loops: Vec<u32>,
    pub envs: Vec<super::env::Env>,
    pub env_vals: Vec<CoreValue>,
    pub loop_items: TripleColumn<CoreValue>,
    pub open_children: Column<Vec<symphonyscript_core::NodeKind>>,
    pub loop_reads: TripleColumn<CoreValue>,
    pub placeholder_uses: Vec<(u32, u32, Span, u32)>,
    pub default_nodes: Column<u32>,
    pub default_insts: PairColumn<u32>,
    pub deferred_uses: Vec<(u32, Vec<u32>)>,
    pub deferred_params: PairSet,
    pub pattern_items: Column<u32>,
    pub own_reads: ColumnSet,
    pub env_rows: PairColumn<u32>,
    pub env_reads: PairColumn<u32>,
    pub use_names: PairColumn<u32>,
    pub use_index: Vec<(u32, u32, u32)>,
    pub group_items: ColumnSet,
    pub path_env: u32,
    pub env_path_rows: PairColumn<u32>,
    pub env_param_reads: Column<Vec<(u32, u32)>>,
    pub sig_index: HashMap<(super::core::BodyKey, Vec<(u32, super::ref_bodies::Binding)>), u32>,
    pub history_target: Vec<u8>,
    pub cell_reads: Vec<super::groups::CellReads>,
    pub cell_group: Vec<u32>,
    pub groups: Vec<Vec<u32>>,
    pub group_reads: Column<(u32, u32)>,
    pub group_kinds: Column<Fact>,
    pub group_variable: ColumnSet,
    pub group_history: Column<Vec<u32>>,
    pub group_open: ColumnSet,
    pub read_log: Vec<u32>,
    pub init_reads: Vec<super::init_read::InitRead>,
    pub init_open: super::init_read::Index,
    pub init_args: super::init_read::Index,
    pub init_serial: u64,
    pub cell_capacities: Column<i128>,
    pub solved_cells: Option<HashSet<u32>>,
    pub solving_cells: bool,
    pub solved_faults: Vec<crate::diagnostics::Diagnostic>,
    pub extent_cache: Column<super::extent::Extent>,
    pub bare: ColumnSet,
    pub unknown_variants: ColumnSet,
    pub family_faults: ColumnSet,
    pub units_seen: bool,
    pub enums_seen: bool,
    pub bare_enums: Column<symphonyscript_core::EnumId>,
    pub untyped_bare: ColumnSet,
    pub takers: Vec<(u32, symphonyscript_core::FamilyId, u32)>,
    pub unit_feeds: Vec<(u32, symphonyscript_core::FamilyId, usize, u32)>,
    pub taker_set: TripleSet,
    pub unit_reaches: TripleColumn<std::rc::Rc<super::unit_reach::UnitReach>>,
    pub unit_let_values: HashMap<(u32, usize, symphonyscript_core::FamilyId, usize), CoreValue>,
    pub unit_conflicted: TripleSet,
    pub cycle_through_refused: bool,
    pub unbound_nodes: ColumnSet,
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
            unflowed: Default::default(),
            unused_reads: Default::default(),
            unexpected_reads: Default::default(),
            compared_reads: Default::default(),
            placeholder_kinds: Default::default(),
            not_values: Default::default(),
            not_values_drawn: Default::default(),
            not_callables_drawn: Default::default(),
            cyclic_let_reads: Default::default(),
            let_seeds: Default::default(),
            pi: Default::default(),
            compared_fits: Vec::new(),
            driving: false,
            need: None,

            rule_reads: vec![[None; 7]; ir.cells.len()],
            cell_order: Vec::new(),
            cell_cyclic: vec![false; ir.cells.len()],
            inst: NONE,
            body_of: vec![NONE; nodes],
            bodies: Vec::new(),
            body_facts: Default::default(),
            body_kept: Default::default(),
            body_reactive: Default::default(),
            call_body: Default::default(),
            call_widths: Default::default(),
            eval_chain: CallChain::default(),
            chain_arena: ChainArena::default(),
            typing_locals: Default::default(),
            reads_cell: vec![None; ir.lets.len()],
            instances: Vec::new(),
            top_instances: Vec::new(),
            store: Store::default(),
            let_values: vec![[None, None, None, None, None, None, None]; ir.lets.len()],
            uses: BTreeMap::new(),
            fold_steps: 0,
            stop: None,
            fold_nodes: vec![NONE; ir.fors.len()],
            warned: Default::default(),
            unknown_kind: false,
            unknown_top: false,
            unknown_wires: false,
            unknown_children: Default::default(),
            skipped_top: Vec::new(),
            reached: vec![false; nodes],
            body_index: HashMap::new(),
            value_index: HashMap::new(),
            keyed_params: None,
            outer_owner: Default::default(),
            call_memo: HashMap::new(),
            recordings: Vec::new(),
            reaches_out: vec![false; ir.fors.len()],
            wires: Vec::new(),
            candidates: Vec::new(),
            row_of_item: vec![NONE; ir.items.len()],
            expanded: 0,
            reference_roots: Vec::new(),
            path_rows: Default::default(),
            paths: vec![NONE; nodes],
            attr_reads: Vec::new(),
            attr_index: Default::default(),
            attr_values: Vec::new(),
            attr_order: Vec::new(),
            field_receivers: Vec::new(),
            child_index: Vec::new(),
            wire_index: Vec::new(),
            wire_attr_index: Default::default(),
            to_wires: Vec::new(),
            body_path_faults: Default::default(),
            deferred_to: Vec::new(),
            untyped_reads: Vec::new(),
            open_reads: Default::default(),
            open_values: Default::default(),
            unrun_to_wires: Vec::new(),
            to_choices: Default::default(),
            wire_now: NONE,
            wire_destinations: Default::default(),
            choosing: NONE,
            feed_env: NONE,
            row_reads: Default::default(),
            choice_reads: Default::default(),
            runtime_to: Default::default(),
            row_choices: Default::default(),
            ref_insts: Default::default(),
            typed_as: Default::default(),
            pattern_files: vec![0; ir.patterns.len()],
            cyclic_patterns: vec![false; ir.patterns.len()],
            named_patterns: vec![true; ir.patterns.len()],
            patterns_used: Default::default(),
            use_links: Vec::new(),
            use_spans: Default::default(),
            use_args: Default::default(),
            use_values: Default::default(),
            compile_time_folds: Vec::new(),
            use_faults: Default::default(),
            default_faults: Default::default(),
            typing_now: 0,
            inst_row: Default::default(),
            owner_insts: Default::default(),
            inst_reads: Default::default(),
            path_owners: None,
            per_inst_paths: Default::default(),
            param_paths: Default::default(),
            param_reads: Vec::new(),
            member_keys: Default::default(),
            array_members: Default::default(),
            expr_members: Default::default(),
            cell_sizes: Default::default(),
            runtime_members: Default::default(),
            capacity_members: Default::default(),
            history_capacities: Default::default(),
            dot_calls: Default::default(),
            bundle_loops: Default::default(),
            unknown_methods: Default::default(),
            function_values: Default::default(),
            path_free: Default::default(),
            child_scopes: Vec::new(),
            scope_of: Default::default(),
            loops: super::loops::loop_items(ir),
            envs: Vec::new(),
            env_vals: Vec::new(),
            loop_items: Default::default(),
            open_children: Default::default(),
            loop_reads: Default::default(),
            placeholder_uses: Vec::new(),
            default_nodes: Default::default(),
            default_insts: Default::default(),
            deferred_uses: Vec::new(),
            deferred_params: Default::default(),
            pattern_items: Default::default(),
            own_reads: Default::default(),
            env_rows: Default::default(),
            env_reads: Default::default(),
            use_names: Default::default(),
            use_index: Vec::new(),
            group_items: Default::default(),
            path_env: NONE,
            env_path_rows: Default::default(),
            env_param_reads: Default::default(),
            sig_index: HashMap::new(),
            history_target: history_targets(ir),
            cell_reads: Vec::new(),
            cell_group: Vec::new(),
            groups: Vec::new(),
            group_reads: Default::default(),
            group_kinds: Default::default(),
            group_variable: Default::default(),
            group_history: Default::default(),
            group_open: Default::default(),
            read_log: Vec::new(),
            init_reads: Vec::new(),
            init_open: HashMap::new(),
            init_args: HashMap::new(),
            init_serial: 0,
            cell_capacities: Default::default(),
            solved_cells: None,
            solving_cells: false,
            solved_faults: Vec::new(),
            extent_cache: Default::default(),
            bare: Default::default(),
            unknown_variants: Default::default(),
            family_faults: Default::default(),
            units_seen: false,
            enums_seen: false,
            bare_enums: Default::default(),
            untyped_bare: Default::default(),
            takers: Vec::new(),
            unit_feeds: Vec::new(),
            taker_set: Default::default(),
            unit_reaches: Default::default(),
            unit_let_values: HashMap::new(),
            unit_conflicted: Default::default(),
            cycle_through_refused: false,
            unbound_nodes: Default::default(),
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

impl Core<'_> {
    pub fn push_call(&mut self, call: u32) {
        self.chain_arena.push(&mut self.eval_chain, call);
    }
}
