#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod ids;
mod language;
mod source;
mod types;
mod vocab;

pub use ids::{
    AttrId, CellId, EnumId, FamilyId, InputSlot, InstanceId, NodeKind, StrId, WireId, WireKind,
};
pub use language::{BUILTINS, FLAG_CHARS, KEYWORDS, RESERVED_WORDS};
pub use source::{Path, Segment, Span};
pub use types::{Ty, Value, canonical_f32, canonical_f64};
pub use vocab::{
    ArrayElement, AttrBuilder, AttrDecl, AttrDefault, AttrType, AttrUnit, Count, EnumAttr,
    EnumBuilder, EnumDecl, FamilyDecl, InvalidVocabulary, NodeKindBuilder, NodeKindDecl, Numeric,
    PreludeFile, UnitDecl, UnitFamilyBuilder, UnitFamilyStart, Vocabulary, VocabularyBuilder,
    VocabularyError, VocabularyStamp, WireKindBuilder, WireKindDecl,
};
