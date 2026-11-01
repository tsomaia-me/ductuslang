use super::instr::Instr;

#[derive(Debug)]
pub(crate) struct Program {
    pub code: Vec<Instr>,
    pub epoch: u32,
}

#[cfg(test)]
impl Program {
    pub fn bytes(&self) -> usize {
        std::mem::size_of::<Program>()
            .saturating_add(self.code.capacity() * std::mem::size_of::<Instr>())
    }
}
