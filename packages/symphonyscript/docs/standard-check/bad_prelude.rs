// The peer's check of the bad form: what it needs around the printed block.
#![allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokKind {
    Name, Number, Or, And, Not, Lt, Le, Gt, Ge, Fallback, Plus, Minus, Without, Star, Slash,
    Div, Percent, Pow, Open, Close, End,
}
#[derive(Clone, Copy, Debug)]
enum Node {
    Leaf(usize),
    Unary(TokKind, usize),
    Binary(TokKind, usize, usize),
}
