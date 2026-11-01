use super::instr::Instr;

#[derive(Debug)]
pub(crate) struct Program {
    pub code: Vec<Instr>,
    pub epoch: u32,
}
