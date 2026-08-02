//! The `Value` enum and its canonical rendering.

/// A runtime cell value. M0's value space is exactly these five word-sized
/// primitives (D2); wider values arrive with the pool that first needs one.
/// The storage seam may pack them to raw words later without changing this API
/// (R2).
///
/// The derived `PartialEq` is **test convenience only**. Commit-time change
/// gating never uses it: the runtime asks "did this value change?" through its
/// single `value_changed()` function, and that function is where float-equality
/// semantics get ruled (divergence probe #2 — IEEE says `NaN != NaN`, which
/// would leave a NaN-holding cell perpetually dirty). Keeping the question in
/// one named place stops `==` from quietly becoming the gate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// 64-bit signed integer.
    I64(i64),
    /// 32-bit signed integer.
    I32(i32),
    /// 64-bit float.
    F64(f64),
    /// 32-bit float.
    F32(f32),
    /// Boolean.
    Bool(bool),
}

impl Value {
    /// The canonical rendering (R24 as amended by R34).
    ///
    /// Ductus *defines* this form. It coincides with Rust's `Display` today, so
    /// the implementation delegates — but the delegation is implementation, not
    /// the definition:
    ///
    /// - integers in decimal;
    /// - bools `true` / `false`;
    /// - floats in shortest-roundtrip decimal, never exponent form, with
    ///   `1.0` → `1`, `-0.0` → `-0`, and the `NaN` / `inf` / `-inf` spellings.
    ///
    /// This is what the trace writes. The tests below pin every case: they are
    /// Ductus's own spec, there to catch upstream drift, not a restatement of
    /// what Rust happens to do.
    ///
    /// Whether the `.duir` emitter writes this same text is a separate,
    /// still-open question, settled at the emitter's increment. The `.duir`
    /// counterpart spellings are `nan`, `inf`, and `-inf` (033-248), admitted
    /// only in a float-typed position; the open question is whether the emitter
    /// writes this trace-canonical text or those spellings.
    ///
    /// Renderings are type-erased: `F64(1.0)` and `I64(1)` both render `1`, so
    /// a rendering never carries its type. The per-cell static type is what
    /// disambiguates.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Value::I64(v) => v.to_string(),
            Value::I32(v) => v.to_string(),
            Value::F64(v) => v.to_string(),
            Value::F32(v) => v.to_string(),
            Value::Bool(v) => v.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_integers_in_decimal() {
        assert_eq!(Value::I64(42).render(), "42");
        assert_eq!(Value::I64(-1).render(), "-1");
        assert_eq!(Value::I64(i64::MIN).render(), "-9223372036854775808");
        assert_eq!(Value::I32(7).render(), "7");
        assert_eq!(Value::I32(i32::MIN).render(), "-2147483648");
    }

    #[test]
    fn renders_bools_as_keywords() {
        assert_eq!(Value::Bool(true).render(), "true");
        assert_eq!(Value::Bool(false).render(), "false");
    }

    // The float cases pin the canonical form spelled out in `render`'s doc:
    // these assertions ARE the specification of what a trace line contains, so
    // they are written out rather than computed.
    #[test]
    fn renders_f64_edges() {
        assert_eq!(Value::F64(1.0).render(), "1");
        assert_eq!(Value::F64(0.1).render(), "0.1");
        assert_eq!(Value::F64(2.5).render(), "2.5");
        assert_eq!(Value::F64(-3.75).render(), "-3.75");
        assert_eq!(Value::F64(0.0).render(), "0");
        assert_eq!(Value::F64(-0.0).render(), "-0");
        assert_eq!(Value::F64(f64::NAN).render(), "NaN");
        assert_eq!(Value::F64(f64::INFINITY).render(), "inf");
        assert_eq!(Value::F64(f64::NEG_INFINITY).render(), "-inf");
    }

    #[test]
    fn renders_f32_edges() {
        assert_eq!(Value::F32(1.0).render(), "1");
        assert_eq!(Value::F32(0.1).render(), "0.1");
        assert_eq!(Value::F32(-2.5).render(), "-2.5");
        assert_eq!(Value::F32(-0.0).render(), "-0");
        assert_eq!(Value::F32(f32::NAN).render(), "NaN");
        assert_eq!(Value::F32(f32::INFINITY).render(), "inf");
        assert_eq!(Value::F32(f32::NEG_INFINITY).render(), "-inf");
    }

    // Large magnitudes stay in plain decimal — the canonical form never
    // switches to exponent notation, so 1e300 is one digit and 300 zeros.
    #[test]
    fn renders_large_magnitudes_without_exponent_form() {
        let rendered = Value::F64(1e300).render();

        assert_eq!(rendered.len(), 301);
        assert!(!rendered.contains('e'));
    }

    // Shortest-roundtrip means "shortest that reads back to the same bits",
    // not "shortest that looks tidy": the classic 0.1 + 0.2 case shows all of
    // it.
    #[test]
    fn renders_the_digits_roundtripping_demands() {
        assert_eq!(Value::F64(0.1 + 0.2).render(), "0.30000000000000004");
    }

    // Same source literal, two widths, two different canonical renderings: an
    // f32 renders the shortest text for f32 precision, and widening that f32
    // to f64 exposes the bits f32 actually holds.
    #[test]
    fn f32_and_a_widened_f32_render_differently() {
        assert_eq!(Value::F32(0.1).render(), "0.1");
        assert_eq!(
            Value::F64(f64::from(0.1_f32)).render(),
            "0.10000000149011612"
        );
    }

    #[test]
    fn rendering_never_carries_the_type() {
        assert_eq!(Value::F64(1.0).render(), Value::I64(1).render());
        assert_eq!(Value::F32(1.0).render(), Value::I32(1).render());
    }

    // Executable form of the doc warning above: derived equality is plain IEEE,
    // which is why change gating gets its own function instead of using `==`.
    #[test]
    fn derived_equality_is_ieee_not_a_change_gate() {
        assert_ne!(Value::F64(f64::NAN), Value::F64(f64::NAN));
        assert_eq!(Value::F64(-0.0), Value::F64(0.0));
    }

    #[test]
    fn distinct_variants_are_never_equal() {
        assert_ne!(Value::I64(1), Value::I32(1));
        assert_ne!(Value::F64(1.0), Value::F32(1.0));
    }
}
