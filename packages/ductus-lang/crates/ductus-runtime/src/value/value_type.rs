//! The `ValueType` enum: the M0 word tier as a type vocabulary.

use ductus_ir::types::Prim;

use super::value::Value;

/// The type a [`Value`] carries — the M0 word tier (SPEC §14.3, D2) as a
/// vocabulary of its own, for dispatch and for diagnostics.
///
/// A vocabulary SCALAR, so `PartialEq` here is not the equality R17 withholds —
/// the `Prim` reading exactly: there is no structure to compare and no identity
/// question to beg, and [`matches`](ValueType::matches)'s dispatch and the
/// tests are what need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// `i64`
    I64,
    /// `i32`
    I32,
    /// `f64`
    F64,
    /// `f32`
    F32,
    /// `bool`
    Bool,
}

impl ValueType {
    /// The type `value` carries.
    #[must_use]
    pub fn of(value: &Value) -> ValueType {
        match value {
            Value::I64(_) => ValueType::I64,
            Value::I32(_) => ValueType::I32,
            Value::F64(_) => ValueType::F64,
            Value::F32(_) => ValueType::F32,
            Value::Bool(_) => ValueType::Bool,
        }
    }

    /// Whether `value` is of this type — [`of`](ValueType::of), compared.
    ///
    /// The `type_mismatched_write` diagnostic is exactly a value this returns
    /// `false` for, at the cell whose declared type is asking.
    #[must_use]
    pub fn matches(&self, value: &Value) -> bool {
        ValueType::of(value) == *self
    }

    /// The M0 word tier as a partial map from the sixteen IR primitive tags.
    ///
    /// `i64`, `i32`, `f64`, `f32` and `bool` map; the other eleven do not.
    /// `None` means "outside the M0 slice", and the loader is what turns that
    /// answer into the `outside_m0_slice` diagnostic — this function just
    /// answers the tier question.
    ///
    /// The rejecting arm lists every non-word tag rather than wildcarding, so a
    /// seventeenth `Prim` variant must decide its tier here before the crate
    /// compiles again.
    #[must_use]
    pub fn from_prim(prim: &Prim) -> Option<ValueType> {
        let value_type = match prim {
            Prim::I64 => ValueType::I64,
            Prim::I32 => ValueType::I32,
            Prim::F64 => ValueType::F64,
            Prim::F32 => ValueType::F32,
            Prim::Bool => ValueType::Bool,
            Prim::I8
            | Prim::I16
            | Prim::I128
            | Prim::U8
            | Prim::U16
            | Prim::U32
            | Prim::U64
            | Prim::U128
            | Prim::Isize
            | Prim::Usize
            | Prim::Str => return None,
        };

        Some(value_type)
    }

    /// The `.duir` spelling of this type — `Prim`'s own word for it — for
    /// diagnostics.
    ///
    /// One direction only, on purpose: nothing needs a `from_word` here,
    /// because the parser already classifies type words through
    /// [`Prim::from_word`] and everything downstream arrives holding a `Prim`.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ValueType::I64 => "i64",
            ValueType::I32 => "i32",
            ValueType::F64 => "f64",
            ValueType::F32 => "f32",
            ValueType::Bool => "bool",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One value of each type, in one place — `matches` walks every pair of
    // them, so the five stay covered as a set.
    const WITNESSES: [(ValueType, Value); 5] = [
        (ValueType::I64, Value::I64(1)),
        (ValueType::I32, Value::I32(1)),
        (ValueType::F64, Value::F64(1.0)),
        (ValueType::F32, Value::F32(1.0)),
        (ValueType::Bool, Value::Bool(true)),
    ];

    #[test]
    fn of_names_the_type_every_variant_carries() {
        assert_eq!(ValueType::of(&Value::I64(-7)), ValueType::I64);
        assert_eq!(ValueType::of(&Value::I32(0)), ValueType::I32);
        assert_eq!(ValueType::of(&Value::F64(f64::NAN)), ValueType::F64);
        assert_eq!(ValueType::of(&Value::F32(-0.0)), ValueType::F32);
        assert_eq!(ValueType::of(&Value::Bool(false)), ValueType::Bool);
    }

    // Every type accepts exactly its own values: 5 accepting pairs, 20
    // refusing ones, no diagonal leaks.
    #[test]
    fn matches_accepts_its_own_type_and_refuses_the_rest() {
        for (value_type, _) in WITNESSES {
            for (other_type, value) in WITNESSES {
                assert_eq!(
                    value_type.matches(&value),
                    value_type == other_type,
                    "{} against a {} value",
                    value_type.as_str(),
                    other_type.as_str()
                );
            }
        }
    }

    // All sixteen primitive tags, written out: the five word-tier tags map, the
    // other eleven answer "outside the M0 slice".
    #[test]
    fn from_prim_maps_exactly_the_word_tier() {
        assert_eq!(ValueType::from_prim(&Prim::I64), Some(ValueType::I64));
        assert_eq!(ValueType::from_prim(&Prim::I32), Some(ValueType::I32));
        assert_eq!(ValueType::from_prim(&Prim::F64), Some(ValueType::F64));
        assert_eq!(ValueType::from_prim(&Prim::F32), Some(ValueType::F32));
        assert_eq!(ValueType::from_prim(&Prim::Bool), Some(ValueType::Bool));

        for prim in [
            Prim::I8,
            Prim::I16,
            Prim::I128,
            Prim::U8,
            Prim::U16,
            Prim::U32,
            Prim::U64,
            Prim::U128,
            Prim::Isize,
            Prim::Usize,
            Prim::Str,
        ] {
            assert_eq!(ValueType::from_prim(&prim), None, "{}", prim.as_str());
        }
    }

    // The spelling is `Prim`'s own — one vocabulary, two crates — so the five
    // mapped pairs agree letter for letter.
    #[test]
    fn as_str_spells_the_duir_word() {
        assert_eq!(ValueType::I64.as_str(), "i64");
        assert_eq!(ValueType::I32.as_str(), "i32");
        assert_eq!(ValueType::F64.as_str(), "f64");
        assert_eq!(ValueType::F32.as_str(), "f32");
        assert_eq!(ValueType::Bool.as_str(), "bool");

        for prim in [Prim::I64, Prim::I32, Prim::F64, Prim::F32, Prim::Bool] {
            let value_type = ValueType::from_prim(&prim).expect("a word-tier tag maps");

            assert_eq!(value_type.as_str(), prim.as_str());
        }
    }
}
