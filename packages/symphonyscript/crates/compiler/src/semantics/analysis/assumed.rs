use super::fast_hash::FastMap;
use super::ids::BindingId;

#[derive(Default)]
pub(crate) struct Assumed {
    pub members: FastMap<BindingId, bool>,
}
