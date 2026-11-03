use super::super::attr_entry_rec::AttrEntryRec;
use super::super::iteration::Iteration;
use super::super::top_rec::TopRec;

#[derive(Clone, Copy, Debug)]
pub(super) enum Value {
    Nothing,
    Expr(u32),
    Pending(Pending),
    Ty {
        root: u32,
        base: u32,
    },
    Arm {
        label: u32,
        value: u32,
        unbuilt: u32,
        lo: u32,
        hi: u32,
    },
    Else(u32),
    Local(Local),
    Named {
        name: u32,
        value: u32,
    },
    Range {
        start: u32,
        end: u32,
    },
    Head(u32),
    Block(u32),
    Item(u32),
    Attr(AttrEntryRec),
    Guard {
        guard: u32,
        block: u32,
    },
    Each {
        head: Iteration,
        members: (u32, u32),
        lo: u32,
        hi: u32,
    },
    Staged {
        start: u32,
        end: u32,
    },
    Members((u32, u32)),
    Param {
        name: u32,
        default: u32,
    },
    Field {
        name: u32,
        ty: u32,
        lo: u32,
        hi: u32,
    },
    Top(TopRec),
    Inline {
        members: (u32, u32),
        regions: (u32, u32),
    },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Pending {
    pub receiver: u32,
    pub receiver_name: u32,
    pub receiver_range: (u32, u32),
    pub receiver_is_name_ref: bool,
    pub receiver_missing: bool,
    pub field: u32,
    pub lo: u32,
    pub hi: u32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Local {
    pub name: u32,
    pub ty: u32,
    pub value: u32,
    pub is_match: bool,
    pub lo: u32,
    pub hi: u32,
}
