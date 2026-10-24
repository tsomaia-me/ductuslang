use super::ident::Ident;
use super::ids::ExprId;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Args {
    Positional(Vec<ExprId>),
    Named(Vec<(Ident, ExprId)>),
}

impl Args {
    pub fn values(&self) -> Vec<ExprId> {
        match self {
            Args::Positional(values) => values.clone(),
            Args::Named(values) => values.iter().map(|(_, value)| *value).collect(),
        }
    }
}
