use symphonyscript_core::{BinaryOp, SpecFloat};

pub(crate) fn float_binary<
    F: SpecFloat
        + std::ops::Add<Output = F>
        + std::ops::Sub<Output = F>
        + std::ops::Mul<Output = F>
        + std::ops::Div<Output = F>,
>(
    op: BinaryOp,
    a: F,
    b: F,
) -> Option<F> {
    Some(match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div => a / b,
        BinaryOp::Rem => a.spec_rem(b),
        BinaryOp::Pow => a.spec_pow(b),
        BinaryOp::IntDiv
        | BinaryOp::Is
        | BinaryOp::IsNot
        | BinaryOp::Lt
        | BinaryOp::Le
        | BinaryOp::Gt
        | BinaryOp::Ge
        | BinaryOp::And
        | BinaryOp::Or => return None,
    })
}
