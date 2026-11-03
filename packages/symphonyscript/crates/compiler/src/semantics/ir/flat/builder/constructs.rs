pub(super) const CONSTRUCTS: [&str; 12] = [
    "a sum type (SPEC §3.9)",
    "a function type (SPEC §6.7)",
    "the string type `str` (SPEC §3.3)",
    "a map type (SPEC §3.10)",
    "the document type `json` (SPEC §3.11)",
    "a parenthesized type (SPEC §3.2)",
    "a qualified name (SPEC §15.3)",
    "a list of bracket arguments of a type (SPEC §6.8)",
    "a size written as an expression (SPEC §3.2)",
    "the type `none` (SPEC §3.9)",
    "a capacity at an inner level of an array type (SPEC §3.2)",
    "an optional type `T?` (SPEC §3.9)",
];

pub(super) const SUM_TYPE: u32 = 0;
pub(super) const FN_TYPE: u32 = 1;
pub(super) const STR_TYPE: u32 = 2;
pub(super) const MAP_TYPE: u32 = 3;
pub(super) const JSON_TYPE: u32 = 4;
pub(super) const PAREN_TYPE: u32 = 5;
pub(super) const QUALIFIED_NAME: u32 = 6;
pub(super) const TYPE_ARGS: u32 = 7;
pub(super) const SIZE: u32 = 8;
pub(super) const NONE_TYPE: u32 = 9;
pub(super) const INNER_CAPACITY: u32 = 10;
pub(super) const OPTIONAL_TYPE: u32 = 11;

pub(super) const LAMBDA: &str = "a lambda (SPEC §6.7)";
pub(super) const LOCAL_ANNOTATION: &str = "an annotation on a local `let` (SPEC §6.3)";
pub(super) const GENERIC: &str = "a list of bracket parameters (SPEC §6.8)";
pub(super) const DOCUMENT_LITERAL: &str = "a document literal (SPEC §3.11)";
