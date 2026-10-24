use super::ids::ExprId;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct MatchArm {
    pub label: ExprId,
    pub value: ExprId,
}
