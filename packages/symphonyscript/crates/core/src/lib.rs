#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod compiled;
mod ids;
mod language;
mod math;
mod source;
mod types;
mod vocab;

pub use compiled::{
    BinaryOp, Binding, Builtin, Cell, Entry, Expr, ExprError, ExprNode, ExprRef, Input, Instance,
    Program, Target, UnaryOp, Wire, dump,
};
pub use ids::{
    AttrId, CellId, EnumId, FamilyId, InputSlot, InstanceId, NodeKind, StrId, WireId, WireKind,
};
pub use language::{BUILTINS, FLAG_CHARS, KEYWORDS, RESERVED_WORDS};
pub use math::{SpecFloat, rand};
pub use source::{Path, Segment, Span};
pub use types::{Scalar, Ty, Value, canonical_f32, canonical_f64};
pub use vocab::{
    ArrayElement, AttrBuilder, AttrDecl, AttrDefault, AttrType, AttrUnit, Count, EnumAttr,
    EnumBuilder, EnumDecl, FamilyDecl, InvalidVocabulary, NodeKindBuilder, NodeKindDecl, Numeric,
    PreludeFile, UnitDecl, UnitFamilyBuilder, UnitFamilyStart, Vocabulary, VocabularyBuilder,
    VocabularyError, VocabularyStamp, WireKindBuilder, WireKindDecl,
};
