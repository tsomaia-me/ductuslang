use crate::semantics::Internal;

pub(crate) fn is_cycle(component: &[usize], edges: &[Vec<usize>]) -> Result<bool, Internal> {
    match component {
        [] => Err(Internal::missed("graph component")),
        [only] => Ok(edges
            .get(*only)
            .ok_or(Internal::missed("graph node"))?
            .contains(only)),
        _ => Ok(true),
    }
}
