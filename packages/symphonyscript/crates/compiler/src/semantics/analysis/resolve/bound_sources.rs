use super::bind_fault::BindFault;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::ir::{Args, Ident, Param};

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
            for (earlier, (name, _)) in values.iter().enumerate() {
                if !params.iter().any(|param| named(param, name)) {
                    return Err(BindFault::Unknown(name));
                }
                if values
                    .iter()
                    .take(earlier)
                    .any(|(other, _)| other.text == name.text)
                {
                    return Err(BindFault::Duplicate(name));
                }
            }
            for (slot, param) in sources.iter_mut().zip(params) {
                *slot = values
                    .iter()
                    .find(|(name, _)| named(param, name))
                    .map(|(_, value)| ParamSource::Arg {
                        expr: *value,
                        scope,
                    });
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

fn named(param: &Param, name: &Ident) -> bool {
    param.name.as_ref().is_some_and(|own| own.text == name.text)
}
