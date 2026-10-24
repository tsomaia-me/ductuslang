use super::kind::Kind;

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
