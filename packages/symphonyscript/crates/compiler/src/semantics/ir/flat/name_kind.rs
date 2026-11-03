#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NameKind {
    Unclassified,
    Reference,
    Declaration,
    ItemKind,
    AsName,
    NamedArgument,
    FieldKey,
    PathSegment,
    AttributeName,
    Binder,
    TypeName,
    Variant,
}
