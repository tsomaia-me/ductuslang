#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Variable {
    Fixed,
    Unknown,
    Whole,
}

impl Variable {
    pub fn either(self, other: Variable) -> Variable {
        match (self, other) {
            (Variable::Whole, _) | (_, Variable::Whole) => Variable::Whole,
            (Variable::Unknown, _) | (_, Variable::Unknown) => Variable::Unknown,
            (Variable::Fixed, Variable::Fixed) => Variable::Fixed,
        }
    }

    pub fn agreed(answers: impl IntoIterator<Item = Variable>) -> Variable {
        let mut answers = answers.into_iter();
        let Some(first) = answers.next() else {
            return Variable::Fixed;
        };
        if answers.all(|answer| answer == first) {
            first
        } else {
            Variable::Unknown
        }
    }
}
