use super::condition::Condition;
use super::ids::ConditionsId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ConditionLink {
    pub condition: Condition,
    pub rest: Option<ConditionsId>,
}
