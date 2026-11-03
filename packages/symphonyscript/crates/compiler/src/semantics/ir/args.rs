use super::ident::Ident;
use super::ids::ExprId;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Args {
    Positional(Vec<ExprId>),
    Named(Vec<(Ident, ExprId)>),
    Received(ExprId, Box<Args>),
    Cut(Box<Args>),
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
            Args::Cut(inner) => inner.values(),
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
            Args::Cut(inner) => inner.positional(),
        }
    }

    pub fn receiver(&self) -> Option<ExprId> {
        match self {
            Args::Received(receiver, _) => Some(*receiver),
            Args::Cut(inner) => inner.receiver(),
            Args::Positional(_) | Args::Named(_) => None,
        }
    }

    pub fn is_cut(&self) -> bool {
        matches!(self, Args::Cut(_))
    }

    pub fn uncut(&self) -> &Args {
        match self {
            Args::Cut(inner) => inner.uncut(),
            Args::Positional(_) | Args::Named(_) | Args::Received(..) => self,
        }
    }
}
