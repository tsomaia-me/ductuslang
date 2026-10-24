use super::ids::ExprId;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum StrPart {
    Text(String),
    Hole(ExprId),
}
