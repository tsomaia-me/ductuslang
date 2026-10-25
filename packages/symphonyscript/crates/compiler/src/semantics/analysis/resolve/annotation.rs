use symphonyscript_core::Scalar;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Annotation {
    pub scalar: Scalar,
    pub lengths: Vec<u32>,
    pub capacity: Option<u32>,
}
