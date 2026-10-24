#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Rem,
    Pow,
    Is,
    IsNot,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinaryOp {
    pub fn name(self) -> &'static str {
        match self {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "sub",
            BinaryOp::Mul => "mul",
            BinaryOp::Div => "div",
            BinaryOp::IntDiv => "intdiv",
            BinaryOp::Rem => "rem",
            BinaryOp::Pow => "pow",
            BinaryOp::Is => "is",
            BinaryOp::IsNot => "isnot",
            BinaryOp::Lt => "lt",
            BinaryOp::Le => "le",
            BinaryOp::Gt => "gt",
            BinaryOp::Ge => "ge",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
        }
    }
}
