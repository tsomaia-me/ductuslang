use symphonyscript_core::Scalar;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Width {
    Flexible,
    Fixed(Scalar),
    Widened(Scalar),
    Adaptive,
}
