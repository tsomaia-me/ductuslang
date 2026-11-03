use crate::syntax::syntax_kind::SyntaxKind;

#[derive(Clone, Copy)]
pub(crate) struct LineToken {
    pub(crate) kind: SyntaxKind,
    pub(crate) start: u32,
    pub(crate) end: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct LineTokens<'t> {
    pub(crate) kinds: &'t [SyntaxKind],
    pub(crate) starts: &'t [u32],
    pub(crate) end: u32,
}

impl<'t> LineTokens<'t> {
    pub(crate) fn iter(self) -> impl Iterator<Item = LineToken> + 't {
        let LineTokens { kinds, starts, end } = self;
        kinds
            .iter()
            .zip(starts)
            .enumerate()
            .map(move |(index, (kind, start))| LineToken {
                kind: *kind,
                start: *start,
                end: starts.get(index + 1).copied().unwrap_or(end),
            })
    }
}

pub(crate) struct LineStart<'t> {
    pub(crate) first: LineToken,
    pub(crate) first_depth: u32,
    pub(crate) rest: LineTokens<'t>,
    pub(crate) rest_depths: &'t [u32],
    pub(crate) top_level: bool,
    pub(crate) closes_region: bool,
}
