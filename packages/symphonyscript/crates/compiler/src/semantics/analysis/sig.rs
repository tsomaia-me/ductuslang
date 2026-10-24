use symphonyscript_core::{EnumId, FamilyId, Scalar};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Kind {
    Int,
    Float,
    Bool,
    Enum(EnumId),
    Str,
    Node,
    Bare,
    Any,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Width {
    Flexible,
    Fixed(Scalar),
    Widened(Scalar),
    Adaptive,
}

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

pub(crate) fn kind_of(scalar: Scalar) -> Kind {
    match scalar {
        Scalar::I32 | Scalar::I64 => Kind::Int,
        Scalar::F32 | Scalar::F64 => Kind::Float,
        Scalar::Bool => Kind::Bool,
        Scalar::Enum(id) => Kind::Enum(id),
        Scalar::Str => Kind::Str,
    }
}

pub(crate) fn is_integer(scalar: Scalar) -> bool {
    matches!(scalar, Scalar::I32 | Scalar::I64)
}

pub(crate) fn is_float(scalar: Scalar) -> bool {
    matches!(scalar, Scalar::F32 | Scalar::F64)
}

pub(crate) fn rank(scalar: Scalar) -> u8 {
    match scalar {
        Scalar::I32 => 0,
        Scalar::I64 => 1,
        Scalar::F32 => 2,
        Scalar::F64 => 3,
        Scalar::Bool | Scalar::Enum(_) | Scalar::Str => 4,
    }
}

pub(crate) fn widens(from: Scalar, to: Scalar) -> bool {
    from == to
        || matches!(
            (from, to),
            (Scalar::I32, Scalar::I64 | Scalar::F32 | Scalar::F64)
                | (Scalar::I64, Scalar::F64)
                | (Scalar::F32, Scalar::F64)
        )
}

pub(crate) fn common(a: Scalar, b: Scalar) -> Option<Scalar> {
    if widens(a, b) {
        return Some(b);
    }
    if widens(b, a) {
        return Some(a);
    }
    let numeric = |scalar| {
        matches!(
            scalar,
            Scalar::I32 | Scalar::I64 | Scalar::F32 | Scalar::F64
        )
    };
    (numeric(a) && numeric(b)).then_some(Scalar::F64)
}

pub(crate) fn float_for(scalar: Scalar) -> Scalar {
    match scalar {
        Scalar::I64 | Scalar::F64 => Scalar::F64,
        Scalar::I32 | Scalar::F32 | Scalar::Bool | Scalar::Enum(_) | Scalar::Str => Scalar::F32,
    }
}

pub(crate) fn meet_flexible(fixed: Scalar, flexible: Kind) -> Scalar {
    if is_integer(fixed) && flexible == Kind::Float {
        float_for(fixed)
    } else {
        fixed
    }
}

pub(crate) fn unify_kinds(a: Kind, b: Kind) -> Option<Kind> {
    match (a, b) {
        (Kind::Error, _) | (_, Kind::Error) => Some(Kind::Error),
        _ if a == b => Some(a),
        (Kind::Any, other) | (other, Kind::Any) => Some(other),
        (Kind::Int, Kind::Float) | (Kind::Float, Kind::Int) => Some(Kind::Float),
        (Kind::Bare, Kind::Enum(id)) | (Kind::Enum(id), Kind::Bare) => Some(Kind::Enum(id)),
        _ => None,
    }
}
