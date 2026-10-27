use symphonyscript_core::AttrId;
use symphonyscript_core::EnumId;
use symphonyscript_core::FamilyId;

use super::bound_slice::BoundSlice;
use super::bound_val::BoundVal;
use super::c_val::CVal;
use super::call_key::CallKey;
use super::destination::Destination;
use super::eval_ty::EvalTy;
use super::fast_hash::FastMap;
use super::ids::{NodeId, ScopeId, WireId};
use super::item_ref::ItemRef;
use super::let_ref::LetRef;
use super::query::Query;
use super::sig::Sig;
use super::val::Val;
use crate::semantics::ir::{ExprId, InputId};

#[derive(Default)]
pub(crate) struct Memo {
    pub exprs: FastMap<(ExprId, ScopeId), Val>,
    pub bare_matches: Vec<NodeId>,
    pub bare_scrutinees: FastMap<NodeId, EnumId>,
    pub bare_enums: FastMap<NodeId, EnumId>,
    pub bare_labels: FastMap<(ExprId, ScopeId), NodeId>,
    pub groups: FastMap<NodeId, NodeId>,
    pub bound_groups: FastMap<Query, NodeId>,
    pub bound_slices: FastMap<(ExprId, ScopeId), Option<BoundSlice>>,
    pub group_list: Vec<NodeId>,
    pub literal_groups: FastMap<(ExprId, ScopeId), NodeId>,
    pub held_reads: super::fast_hash::FastSet<NodeId>,
    pub lets: FastMap<(ScopeId, LetRef), BoundVal>,
    pub params: FastMap<(ScopeId, u32), BoundVal>,
    pub inputs: FastMap<InputId, BoundVal>,
    pub states: FastMap<crate::semantics::ir::StateId, BoundVal>,
    pub state_exprs: Option<super::fast_hash::FastSet<ExprId>>,
    pub own_history: super::fast_hash::FastSet<NodeId>,
    pub reads_cell: FastMap<crate::semantics::ir::FnId, bool>,
    pub layouts: FastMap<NodeId, (super::resolve::LayoutFound, u32)>,
    pub settle_epoch: u32,
    pub binding_capacities: FastMap<super::ids::BindingId, u32>,
    pub fixed_annotations: super::fast_hash::FastSet<super::ids::BindingId>,
    pub bounded_roots: FastMap<(crate::semantics::ir::FnId, super::ids::ScopeId), NodeId>,
    pub fallback_joins: super::fast_hash::FastSet<NodeId>,
    pub deferred_sums: super::fast_hash::FastSet<NodeId>,
    pub variable_defaults: FastMap<NodeId, (Vec<u32>, u32)>,
    pub default_arrays: FastMap<(ItemRef, AttrId), NodeId>,
    pub passed_over_calls: super::fast_hash::FastSet<(ExprId, ScopeId)>,
    pub own_marked: super::fast_hash::FastSet<crate::semantics::ir::StateId>,
    pub kind_components: FastMap<NodeId, Option<u32>>,
    pub kind_groups: Vec<Vec<NodeId>>,
    pub history_kinds: FastMap<NodeId, Option<Sig>>,
    pub provisional_reads: Vec<NodeId>,
    pub unrun_bodies:
        super::fast_hash::FastSet<(crate::semantics::ir::Loc, Option<crate::semantics::ir::Loc>)>,
    pub capacity_rounds: u64,
    pub settlement_visits: u64,
    pub layout_visits: u64,
    pub blocked_walks: FastMap<crate::semantics::ir::StateId, (u32, crate::semantics::ir::StateId)>,
    pub evaluation_work: u64,
    pub conversions: FastMap<EvalTy, FastMap<*const (), (super::c_array::CArray, CVal)>>,
    pub cycle_work: u64,
    pub reaching_reads: Vec<bool>,
    pub read_walk: u64,
    pub read_free_widths: FastMap<NodeId, Option<symphonyscript_core::Scalar>>,
    pub string_bytes: FastMap<NodeId, NodeId>,
    pub lengths: FastMap<NodeId, Val>,
    pub from_ends: FastMap<(ExprId, super::ids::ScopeId), NodeId>,
    pub boxings: FastMap<(Vec<NodeId>, Vec<u32>), NodeId>,
    pub binding_layouts: FastMap<super::ids::BindingId, super::layout::Layout>,
    pub settled: super::fast_hash::FastSet<super::ids::BindingId>,
    pub float_inputs: super::fast_hash::FastSet<InputId>,
    pub attrs: FastMap<(ItemRef, AttrId), BoundVal>,
    pub destinations: FastMap<WireId, Destination>,
    pub sigs: FastMap<NodeId, Sig>,
    pub evals: FastMap<(NodeId, EvalTy), CVal>,
    pub before_unit_reading: FastMap<(NodeId, EvalTy), CVal>,
    pub call_scopes: FastMap<(ExprId, ScopeId), ScopeId>,
    pub prefix_scopes: FastMap<(ScopeId, u32), ScopeId>,
    pub pattern_bodies: FastMap<ScopeId, ScopeId>,
    pub takes_family: FastMap<NodeId, FamilyId>,
    pub default_scales: FastMap<NodeId, f64>,
    pub calls: FastMap<CallKey, CVal>,
    pub arg_sigs: FastMap<NodeId, Sig>,
    pub templates: FastMap<super::template_key::TemplateKey, ScopeId>,
    pub template_feeds: FastMap<ScopeId, Vec<super::feed::Feed>>,
    pub template_takers: FastMap<ScopeId, Vec<NodeId>>,
    pub rec_families: FastMap<super::family_key::FamilyKey, super::rec_family::RecFamily>,
    pub probing_families: super::fast_hash::FastSet<super::family_key::FamilyKey>,
    pub probes: super::fast_hash::FastSet<ScopeId>,
    pub template_base: FastMap<ScopeId, usize>,
    pub template_parents: FastMap<ScopeId, Vec<ScopeId>>,
    pub family_scopes: FastMap<super::family_key::FamilyKey, Vec<ScopeId>>,
    pub key_reach: FastMap<CallKey, (Vec<u32>, Vec<ScopeId>)>,
    pub fault_sites: FastMap<u32, Vec<NodeId>>,
    pub template_sites: FastMap<ScopeId, Vec<NodeId>>,
    pub named_bodies: super::fast_hash::FastSet<crate::semantics::ir::FnId>,
    pub named_scopes: FastMap<crate::semantics::ir::FnId, ScopeId>,
    pub cyclic_lets: super::fast_hash::FastSet<crate::semantics::ir::LetId>,
    pub call_feeds: FastMap<NodeId, Vec<super::feed::Feed>>,
    pub shapes: FastMap<NodeId, super::shape_need::ShapeNeed>,
    pub projected_indexes: super::fast_hash::FastSet<NodeId>,
    pub attribute_projections: super::fast_hash::FastSet<NodeId>,
    pub defaulted: FastMap<NodeId, Vec<super::ids::BindingId>>,
    pub boxes: FastMap<(ExprId, ScopeId), NodeId>,
    pub projections: FastMap<NodeId, Vec<NodeId>>,
    pub unknown_bodies:
        FastMap<crate::semantics::ir::BlockId, std::rc::Rc<crate::semantics::ir::UnknownNames>>,
    pub named_sites: FastMap<
        crate::semantics::ir::BlockId,
        std::rc::Rc<[(super::site::Site, crate::semantics::ir::Loc)]>,
    >,
    pub pattern_names: FastMap<
        crate::semantics::ir::PatternId,
        std::rc::Rc<std::collections::BTreeMap<String, super::local_name::LocalName>>,
    >,
    pub local_names: FastMap<
        crate::semantics::ir::FnId,
        std::rc::Rc<std::collections::BTreeMap<String, super::local_name::LocalName>>,
    >,
}
