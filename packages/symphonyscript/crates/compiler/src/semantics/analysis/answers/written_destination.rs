use crate::semantics::Internal;
use crate::semantics::ir::flat::{FlatIr, Op};

pub(crate) enum Written {
    NoInstance,
    Name,
    Choice { test: u32, arms: Vec<u32> },
    Element { base: u32 },
    Elements(Vec<u32>),
    Run,
}

pub(crate) fn written_destination(ir: &FlatIr, node: u32) -> Result<Written, Internal> {
    let (op, a, b, c) = ir.node(node)?;
    Ok(match op {
        Op::Int | Op::Float | Op::Bool | Op::Str | Op::Variant => Written::NoInstance,
        Op::Name => Written::Name,
        Op::If => Written::Choice {
            test: a,
            arms: vec![b, c],
        },
        Op::Index => Written::Element { base: a },
        Op::Array => Written::Elements((a..a + b).map(|at| ir.kid(at)).collect::<Result<_, _>>()?),
        Op::To
        | Op::Convert
        | Op::Field
        | Op::IndexFromEnd
        | Op::Slice
        | Op::Call
        | Op::NamedCall
        | Op::MethodCall
        | Op::NamedMethodCall
        | Op::Past
        | Op::Stepped
        | Op::Neg
        | Op::Not
        | Op::Add
        | Op::Sub
        | Op::Mul
        | Op::Div
        | Op::IntDiv
        | Op::Rem
        | Op::Pow
        | Op::And
        | Op::Or
        | Op::Lt
        | Op::Le
        | Op::Gt
        | Op::Ge
        | Op::Is
        | Op::IsNot
        | Op::Match
        | Op::For
        | Op::TyScalar
        | Op::TyName
        | Op::TyArray
        | Op::TyVar
        | Op::Bad => Written::Run,
    })
}
