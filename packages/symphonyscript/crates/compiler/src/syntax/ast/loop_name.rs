use super::name::Name;
use super::r#type::Type;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LoopName<'p> {
    pub name: Name<'p>,
    pub ty: Option<Type<'p>>,
}
