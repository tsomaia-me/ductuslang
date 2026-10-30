use super::reads::Reads;
use super::variable::Variable;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Answer {
    pub reads: Reads,
    pub variable: Variable,
}

impl Answer {
    pub const VALUE: Answer = Answer {
        reads: Reads::Value,
        variable: Variable::Fixed,
    };

    pub fn of(reads: Reads, variable: Variable) -> Answer {
        Answer { reads, variable }
    }

    pub fn join(self, other: Answer) -> Answer {
        Answer {
            reads: self.reads.join(other.reads),
            variable: self.variable.either(other.variable),
        }
    }
}
