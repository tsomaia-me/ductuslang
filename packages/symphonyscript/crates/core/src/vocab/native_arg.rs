use crate::types::Value;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NativeArg<'a> {
    Value(Value),
    Elements(&'a [Value]),
    Text(&'a str),
}
