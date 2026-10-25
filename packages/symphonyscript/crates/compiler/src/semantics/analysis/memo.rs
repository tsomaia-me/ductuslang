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
    pub groups: FastMap<NodeId, NodeId>,
    pub bound_groups: FastMap<Query, NodeId>,
    pub bound_slices: FastMap<(ExprId, ScopeId), Option<BoundSlice>>,
    pub group_list: Vec<NodeId>,
    pub literal_groups: FastMap<(ExprId, ScopeId), NodeId>,
    pub held_reads: super::fast_hash::FastSet<NodeId>,
    pub lets: FastMap<(ScopeId, LetRef), BoundVal>,
    pub params: FastMap<(ScopeId, u32), BoundVal>,
    pub inputs: FastMap<InputId, BoundVal>,
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
    pub call_feeds: FastMap<NodeId, Vec<super::feed::Feed>>,
    pub shapes: FastMap<NodeId, super::shape_need::ShapeNeed>,
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
