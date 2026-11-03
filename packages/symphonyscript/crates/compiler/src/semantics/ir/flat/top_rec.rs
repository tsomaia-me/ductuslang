#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TopRec {
    Let(u32),
    Cell(u32),
    Fn(u32),
    Pattern(u32),
    Placement(u32),
    Placeholder { name: u32, drawn: (u32, u32) },
    Drawn((u32, u32)),
    Skip,
}
