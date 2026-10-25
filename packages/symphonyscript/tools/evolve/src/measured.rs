use crate::work::Work;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Measured {
    pub distinct_codes: Vec<String>,
    pub work: Work,
}
