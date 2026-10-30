use super::ident::Ident;
use super::ids::ExprId;
use super::iterable::Iterable;
use super::let_decl::LetDecl;
use super::loop_var::LoopVar;
use super::type_ref::TypeRef;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct ForExpr {
    pub names: Vec<LoopVar>,
    pub iterable: Iterable,
    pub accumulator: Option<Ident>,
    pub accumulator_ty: Option<TypeRef>,
    pub start: ExprId,
    pub lets: Vec<LetDecl>,
    pub body: ExprId,
}

impl ForExpr {
    pub fn children(&self) -> Vec<ExprId> {
        let mut children = match self.iterable {
            Iterable::Range(start, end) => vec![start, end],
            Iterable::Values(values) => vec![values],
        };
        children.push(self.start);
        children.extend(self.lets.iter().map(|local| local.value));
        children.push(self.body);
        children
    }
}
