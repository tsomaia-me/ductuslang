use crate::semantics::analysis::param_source::ParamSource;

pub(super) fn defaulted(sources: &[ParamSource]) -> Vec<usize> {
    sources
        .iter()
        .enumerate()
        .filter(|(_, source)| matches!(source, ParamSource::Default))
        .map(|(position, _)| position)
        .collect()
}
