use super::name::Name;
use super::type_ref::TypeRef;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LoopName<'p> {
    pub name: Name<'p>,
    pub ty: Option<TypeRef<'p>>,
}
