#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpecExample {
    pub section: String,
    pub tag: String,
    pub ordinal: usize,
    pub line: usize,
    pub text: String,
}

impl SpecExample {
    pub fn name(&self) -> String {
        format!("spec-{}-{}-{}", self.section, self.tag, self.ordinal)
    }
}
