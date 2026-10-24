use super::bind_fault::BindFault;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::ir::{Args, Param};

pub(super) fn bound_sources<'a>(
    params: &[Param],
    args: &'a Args,
    scope: ScopeId,
) -> Result<Vec<ParamSource>, BindFault<'a>> {
    let mut sources: Vec<Option<ParamSource>> = vec![None; params.len()];
    match args {
        Args::Positional(values) => {
            if values.len() > params.len() {
                return Err(BindFault::TooMany {
                    given: values.len(),
                });
            }
            for (slot, value) in sources.iter_mut().zip(values) {
                *slot = Some(ParamSource::Arg {
                    expr: *value,
                    scope,
                });
            }
        }
        Args::Named(values) => {
            for (name, value) in values {
                let position = params
                    .iter()
                    .position(|param| param.name.as_ref().is_some_and(|own| own.text == name.text))
                    .ok_or(BindFault::Unknown(name))?;
                if let Some(slot) = sources.get_mut(position) {
                    if slot.is_some() {
                        return Err(BindFault::Duplicate(name));
                    }
                    *slot = Some(ParamSource::Arg {
                        expr: *value,
                        scope,
                    });
                }
            }
        }
    }
    Ok(params
        .iter()
        .zip(sources)
        .map(|(param, slot)| match slot {
            Some(source) => source,
            None if param.default.is_some() => ParamSource::Default,
            None => ParamSource::Missing,
        })
        .collect())
}
