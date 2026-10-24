use alloc::string::String;

#[derive(Clone, PartialEq, Debug)]
pub struct UnitDecl {
    pub name: String,
    pub scale: f64,
    pub doc: Option<String>,
}
