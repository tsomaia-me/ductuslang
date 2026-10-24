#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum FrameKind {
    Locals,
    Body,
    ForVar,
}
