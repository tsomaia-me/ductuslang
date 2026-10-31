use crate::syntax::TokenView;

ast_node!(JsonAtom);

impl<'p> JsonAtom<'p> {
    pub fn tokens(&self) -> impl Iterator<Item = TokenView<'p>> + use<'p> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| !token.kind().is_trivia())
    }
}
