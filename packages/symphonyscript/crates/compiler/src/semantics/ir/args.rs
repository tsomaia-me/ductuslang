use super::ident::Ident;
use super::ids::ExprId;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Args {
    Positional(Vec<ExprId>),
    Named(Vec<(Ident, ExprId)>),
    Received(ExprId, Box<Args>),
}

impl Args {
    pub fn values(&self) -> Vec<ExprId> {
        match self {
            Args::Positional(values) => values.clone(),
            Args::Named(values) => values.iter().map(|(_, value)| *value).collect(),
            Args::Received(receiver, rest) => {
                let mut values = vec![*receiver];
                values.extend(rest.values());
                values
            }
        }
    }

    pub fn positional(&self) -> Option<Vec<ExprId>> {
        match self {
            Args::Positional(values) => Some(values.clone()),
            Args::Named(_) => None,
            Args::Received(receiver, rest) => {
                let mut values = vec![*receiver];
                values.extend(rest.positional()?);
                Some(values)
            }
        }
    }

    pub fn receiver(&self) -> Option<ExprId> {
        match self {
            Args::Received(receiver, _) => Some(*receiver),
            Args::Positional(_) | Args::Named(_) => None,
        }
    }
}
