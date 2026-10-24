use super::suspension::Suspension;

pub(crate) struct Layout {
    pub(crate) depths: Vec<u32>,
    pub(crate) suspended: Vec<Suspension>,
}
