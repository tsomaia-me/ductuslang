use crate::semantics::analysis::common::common;
use crate::semantics::analysis::float_for::float_for;
use crate::semantics::analysis::meet_flexible::meet_flexible;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;

pub(super) fn arith_width(a: &Sig, b: &Sig) -> Width {
    let settled = |width| match width {
        Width::Widened(scalar) => Width::Fixed(float_for(scalar)),
        other => other,
    };
    match (settled(a.width), settled(b.width)) {
        (Width::Adaptive, _) | (_, Width::Adaptive) => Width::Adaptive,
        (Width::Fixed(x), Width::Fixed(y)) => Width::Fixed(common(x, y).unwrap_or(x)),
        (Width::Fixed(x), Width::Flexible) => Width::Fixed(meet_flexible(x, b.kind)),
        (Width::Flexible, Width::Fixed(y)) => Width::Fixed(meet_flexible(y, a.kind)),
        (Width::Flexible, Width::Flexible) => Width::Flexible,
        (Width::Widened(x), _) | (_, Width::Widened(x)) => Width::Fixed(float_for(x)),
    }
}
