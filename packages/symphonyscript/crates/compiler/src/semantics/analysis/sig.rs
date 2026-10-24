use symphonyscript_core::FamilyId;

use super::kind::Kind;
use super::width::Width;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Sig {
    pub kind: Kind,
    pub width: Width,
    pub reactive: bool,
    pub family: Option<FamilyId>,
    pub frame: bool,
    pub open: bool,
}

impl Sig {
    pub const ERROR: Sig = Sig {
        kind: Kind::Error,
        width: Width::Flexible,
        reactive: false,
        family: None,
        frame: false,
        open: false,
    };

    pub fn is_numeric(&self) -> bool {
        matches!(self.kind, Kind::Int | Kind::Float | Kind::Any)
    }

    pub fn is_error(&self) -> bool {
        self.kind == Kind::Error
    }

    pub fn of(kind: Kind, width: Width, reactive: bool, family: Option<FamilyId>) -> Sig {
        Sig {
            kind,
            width,
            reactive,
            family,
            frame: false,
            open: false,
        }
    }

    pub fn open(reactive: bool) -> Sig {
        Sig {
            kind: Kind::Any,
            width: Width::Flexible,
            reactive,
            family: None,
            frame: false,
            open: true,
        }
    }
}
