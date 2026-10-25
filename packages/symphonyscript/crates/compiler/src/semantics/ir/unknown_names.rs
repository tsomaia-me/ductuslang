#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct UnknownNames {
    pub every: bool,
    pub names: Vec<String>,
}

impl UnknownNames {
    pub fn of(mut names: Vec<String>) -> Self {
        names.sort();
        names.dedup();
        Self {
            every: false,
            names,
        }
    }

    pub fn holds(&self, name: &str) -> bool {
        self.every
            || self
                .names
                .binary_search_by(|probe| probe.as_str().cmp(name))
                .is_ok()
    }

    pub fn is_empty(&self) -> bool {
        !self.every && self.names.is_empty()
    }

    pub fn join(&mut self, other: &UnknownNames) {
        if other.is_empty() {
            return;
        }
        self.every |= other.every;
        self.names.extend(other.names.iter().cloned());
        self.names.sort();
        self.names.dedup();
    }
}
