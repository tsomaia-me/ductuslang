use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub doc: Option<String>,
}
