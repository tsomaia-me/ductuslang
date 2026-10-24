#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Internal {
    pub missed: &'static str,
}

impl Internal {
    pub fn missed(missed: &'static str) -> Internal {
        Internal { missed }
    }
}
