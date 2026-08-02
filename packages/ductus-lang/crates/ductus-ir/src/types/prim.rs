//! The `Prim` enum: the closed set of primitive type tags.

/// A primitive type tag — the type-erased primitives the runtime understands
/// directly (§4.1, §15.4.6).
///
/// The set is CLOSED at exactly these sixteen (IR_GRAMMAR §2's `PRIM`).
/// Aggregates and closures are not primitives: they are built on top by
/// [`TypeTag`](crate::types::TypeTag), which carries `Prim` as one alternative
/// among nine.
///
/// Ductus has no primitive-widening or primitive-aliasing rules at this layer,
/// so the enum needs no ordering and no width arithmetic. It is a name for a
/// spelling, and [`as_str`](Prim::as_str) / [`from_word`](Prim::from_word) are
/// the two directions of that one job.
///
/// `Copy` because a tag is a discriminant — it is cloned constantly while a
/// recursive `TypeTag` is walked, and a by-value copy is the whole value.
///
/// `PartialEq` here is not the equality R17 withholds. R17 targets COMPOSITE
/// AST nodes — a `TypeTag`, a `Module` — whose derived `==` would quietly
/// become the type-identity rule that belongs to the loader. `Prim` is a
/// vocabulary SCALAR: a name for one of sixteen spellings, with no structure to
/// compare and no identity question to beg. Equality here is what
/// [`from_word`](Prim::from_word)'s dispatch and its tests require.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prim {
    /// `i8`
    I8,
    /// `i16`
    I16,
    /// `i32`
    I32,
    /// `i64`
    I64,
    /// `i128`
    I128,
    /// `u8`
    U8,
    /// `u16`
    U16,
    /// `u32`
    U32,
    /// `u64`
    U64,
    /// `u128`
    U128,
    /// `isize`
    Isize,
    /// `usize`
    Usize,
    /// `f32`
    F32,
    /// `f64`
    F64,
    /// `bool`
    Bool,
    /// `str`
    Str,
}

impl Prim {
    /// The one spelling of this primitive in `.duir` text (§2's `PRIM`).
    ///
    /// The emitter writes this and nothing else, which is what makes the
    /// spelling canonical rather than conventional.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Prim::I8 => "i8",
            Prim::I16 => "i16",
            Prim::I32 => "i32",
            Prim::I64 => "i64",
            Prim::I128 => "i128",
            Prim::U8 => "u8",
            Prim::U16 => "u16",
            Prim::U32 => "u32",
            Prim::U64 => "u64",
            Prim::U128 => "u128",
            Prim::Isize => "isize",
            Prim::Usize => "usize",
            Prim::F32 => "f32",
            Prim::F64 => "f64",
            Prim::Bool => "bool",
            Prim::Str => "str",
        }
    }

    /// The primitive `word` names, or `None` when it names none.
    ///
    /// This is the SOFT-KEYWORD classifier for type positions (R46). The lexer
    /// knows no keywords, so `i32` arrives as an ordinary `Ident` exactly like a
    /// cell named `i32` does; a word becomes a primitive only where a type tag
    /// is what the grammar expects, and this function is that decision.
    ///
    /// The rejection is exact, never prefix-based or shape-based: `i7`, `int`,
    /// `f16` and `I32` are all `None`. The set is closed (§2), so a spelling
    /// that is not one of the sixteen is not a primitive of some other width —
    /// it is not a primitive.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Prim> {
        let prim = match word {
            "i8" => Prim::I8,
            "i16" => Prim::I16,
            "i32" => Prim::I32,
            "i64" => Prim::I64,
            "i128" => Prim::I128,
            "u8" => Prim::U8,
            "u16" => Prim::U16,
            "u32" => Prim::U32,
            "u64" => Prim::U64,
            "u128" => Prim::U128,
            "isize" => Prim::Isize,
            "usize" => Prim::Usize,
            "f32" => Prim::F32,
            "f64" => Prim::F64,
            "bool" => Prim::Bool,
            "str" => Prim::Str,
            _ => return None,
        };

        Some(prim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // The closed set, written out. A seventeenth variant added without a
    // spelling fails to compile here, and one added WITH a spelling fails the
    // count assertion below until it is listed.
    const ALL: [Prim; 16] = [
        Prim::I8,
        Prim::I16,
        Prim::I32,
        Prim::I64,
        Prim::I128,
        Prim::U8,
        Prim::U16,
        Prim::U32,
        Prim::U64,
        Prim::U128,
        Prim::Isize,
        Prim::Usize,
        Prim::F32,
        Prim::F64,
        Prim::Bool,
        Prim::Str,
    ];

    #[test]
    fn every_primitive_spells_its_grammar_word() {
        assert_eq!(Prim::I8.as_str(), "i8");
        assert_eq!(Prim::I16.as_str(), "i16");
        assert_eq!(Prim::I32.as_str(), "i32");
        assert_eq!(Prim::I64.as_str(), "i64");
        assert_eq!(Prim::I128.as_str(), "i128");
        assert_eq!(Prim::U8.as_str(), "u8");
        assert_eq!(Prim::U16.as_str(), "u16");
        assert_eq!(Prim::U32.as_str(), "u32");
        assert_eq!(Prim::U64.as_str(), "u64");
        assert_eq!(Prim::U128.as_str(), "u128");
        assert_eq!(Prim::Isize.as_str(), "isize");
        assert_eq!(Prim::Usize.as_str(), "usize");
        assert_eq!(Prim::F32.as_str(), "f32");
        assert_eq!(Prim::F64.as_str(), "f64");
        assert_eq!(Prim::Bool.as_str(), "bool");
        assert_eq!(Prim::Str.as_str(), "str");
    }

    // The two directions are inverses, and no two primitives share a spelling.
    #[test]
    fn from_word_inverts_as_str_for_every_primitive() {
        let mut words = HashSet::new();

        for prim in ALL {
            assert_eq!(Prim::from_word(prim.as_str()), Some(prim));
            assert!(words.insert(prim.as_str()), "{} twice", prim.as_str());
        }

        assert_eq!(words.len(), 16);
    }

    // The set is closed, so a plausible-looking width is not a primitive: `i7`
    // is rejected exactly as `banana` is.
    #[test]
    fn from_word_rejects_what_is_not_one_of_the_sixteen() {
        for word in [
            "i7", "i0", "i256", "f16", "f128", "int", "uint", "string", "char", "I32", "Str",
            "i32 ", " i32", "i32x", "isize_", "", "_", "bool_", "banana",
        ] {
            assert_eq!(Prim::from_word(word), None, "{word}");
        }
    }
}
