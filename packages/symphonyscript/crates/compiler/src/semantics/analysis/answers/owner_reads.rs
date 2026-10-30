use super::answer::Answer;
use super::owners::Owners;

pub(crate) struct OwnerReads {
    pub owners: Owners,
    pub answers: Vec<Answer>,
    pub stamp: Option<(u32, usize)>,
}
