use alloc::string::String;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PreludeFile {
    pub path: String,
    pub text: String,
}
