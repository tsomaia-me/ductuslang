use symphonyscript_core::NodeKind;

#[derive(Clone, Copy, Debug)]
pub(crate) struct InstanceRow {
    pub kind: NodeKind,
    pub item: u32,
    pub file: u32,
    pub children: (u32, u32),
    pub env: u32,
    pub name: u32,
    pub name_file: u32,
    pub reachable: bool,
}
