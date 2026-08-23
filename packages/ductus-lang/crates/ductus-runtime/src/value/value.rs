//! The `Value` enum: a runtime cell value and its canonical rendering.

use std::fmt::Write;

/// A runtime cell value. M0's value space is exactly these five word-sized
/// primitives (R2, D2) — the word tier of SPEC §14.3 — and the one vocabulary
/// the behavior ABI, the store, and the trace all speak. The storage seam may
/// pack them to raw words in a later milestone without changing this API.
///
/// # Why there is no `PartialEq`
///
/// The engine's value-change gating (§13.10.2 step 1) asks "did this value
/// change?", and for floats the answer must be BITWISE: IEEE `==` would make a
/// NaN-holding cell perpetually dirty (`NaN != NaN`), and would conflate `-0.0`
/// with `0.0` — two values the canonical rendering distinguishes. A derived
/// `PartialEq` would be that IEEE trap sitting one `==` away from every engine
/// file, so the comparison exists ONLY as the named method,
/// [`bits_eq`](Value::bits_eq), and reaching for it is a decision the code
/// spells out.
#[derive(Debug, Clone, Copy)]
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
    /// Same variant AND same bits — the value-change-gating comparison
    /// (§13.10.2 step 1).
    ///
    /// Integers and bools compare by value; `F64`/`F32` compare via
    /// [`f64::to_bits`]/[`f32::to_bits`] equality. So `NaN` IS equal to `NaN`
    /// here when the bit patterns match — a NaN-holding cell settles instead of
    /// staying perpetually dirty — while two NaNs with different payload bits
    /// are NOT equal: the contract is bit-pattern fidelity, never NaN-class
    /// equality. `-0.0` and `0.0` are different bits, so they are different
    /// values, exactly as their renderings say. Different variants are never
    /// equal — `I64(1)` and `I32(1)` are two values of two types.
    #[must_use]
    pub fn bits_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::I64(left), Value::I64(right)) => left == right,
            (Value::I32(left), Value::I32(right)) => left == right,
            (Value::F64(left), Value::F64(right)) => left.to_bits() == right.to_bits(),
            (Value::F32(left), Value::F32(right)) => left.to_bits() == right.to_bits(),
            (Value::Bool(left), Value::Bool(right)) => left == right,
            _ => false,
        }
    }

    /// The canonical value rendering (R24 as amended by R34).
    ///
    /// Ductus DEFINES this form; today it coincides with Rust's `Display`, so
    /// the implementation delegates — but the delegation is implementation,
    /// never the definition:
    ///
    /// - integers in decimal;
    /// - bools `true` / `false`;
    /// - floats in shortest-roundtrip decimal, never exponent form, with
    ///   `1.0` → `1`, `-0.0` → `-0`, and the `NaN` / `inf` / `-inf` spellings.
    ///
    /// This is what the trace writes, and the pinned tests below are Ductus's
    /// own spec of it — there to catch upstream Rust drift, not to restate what
    /// Rust happens to do.
    ///
    /// Renderings are TYPE-ERASED: `F64(1.0)` and `I64(1)` both render `1`, so
    /// a rendering never carries its type; the per-cell static type is what
    /// disambiguates.
    #[must_use]
    pub fn render(&self) -> String {
        let mut rendered = String::new();

        self.render_into(&mut rendered);

        rendered
    }

    /// The same canonical rendering, APPENDED to a caller's sink.
    ///
    /// A value is almost never the whole of a line — it sits inside a trace
    /// event the caller is already building — so a caller with a sink of its
    /// own pays no `String` per value.
    pub fn render_into(&self, out: &mut String) {
        // `write!` to a `String` cannot fail; the discarded `Result` is the
        // price of delegating to each payload's `Display`.
        let _ = match self {
            Value::I64(payload) => write!(out, "{payload}"),
            Value::I32(payload) => write!(out, "{payload}"),
            Value::F64(payload) => write!(out, "{payload}"),
            Value::F32(payload) => write!(out, "{payload}"),
            Value::Bool(payload) => write!(out, "{payload}"),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_integers_in_decimal() {
        assert_eq!(Value::I64(0).render(), "0");
        assert_eq!(Value::I64(-7).render(), "-7");
        assert_eq!(Value::I64(i64::MIN).render(), "-9223372036854775808");
        assert_eq!(Value::I32(0).render(), "0");
        assert_eq!(Value::I32(-7).render(), "-7");
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
    fn renders_f64_edges_by_the_pinned_canon() {
        assert_eq!(Value::F64(1.0).render(), "1");
        assert_eq!(Value::F64(-0.0).render(), "-0");
        assert_eq!(Value::F64(0.1).render(), "0.1");
        assert_eq!(Value::F64(1.5).render(), "1.5");
        assert_eq!(Value::F64(f64::NAN).render(), "NaN");
        assert_eq!(Value::F64(f64::INFINITY).render(), "inf");
        assert_eq!(Value::F64(f64::NEG_INFINITY).render(), "-inf");
    }

    // Shortest-roundtrip means "shortest that reads back to the same bits",
    // not "shortest that looks tidy": the classic 0.1 + 0.2 case shows all of
    // it.
    #[test]
    fn renders_the_digits_roundtripping_demands() {
        assert_eq!(Value::F64(0.1 + 0.2).render(), "0.30000000000000004");
    }

    #[test]
    fn renders_f32_edges_by_the_pinned_canon() {
        assert_eq!(Value::F32(1.0).render(), "1");
        assert_eq!(Value::F32(-0.0).render(), "-0");
        assert_eq!(Value::F32(0.1).render(), "0.1");
        assert_eq!(Value::F32(1.5).render(), "1.5");
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

    // The other end of the same law. Rust chooses notation independently at
    // each end, so the large-magnitude pin above does not cover this one: the
    // canon never switches to exponent form at EITHER end, subnormals
    // included.
    #[test]
    fn renders_small_magnitudes_without_exponent_form() {
        let smallest_normal_f64 = Value::F64(1e-300).render();
        let smallest_subnormal_f64 = Value::F64(5e-324).render();
        let small_f32 = Value::F32(1e-38).render();

        assert_eq!(smallest_normal_f64.len(), 302);
        assert!(!smallest_normal_f64.contains('e'));

        assert_eq!(smallest_subnormal_f64.len(), 326);
        assert!(!smallest_subnormal_f64.contains('e'));

        assert_eq!(small_f32.len(), 40);
        assert!(!small_f32.contains('e'));
    }

    #[test]
    fn rendering_never_carries_the_type() {
        assert_eq!(Value::F64(1.0).render(), Value::I64(1).render());
        assert_eq!(Value::F32(1.0).render(), Value::I32(1).render());
    }

    // `render_into` is the rendering and `render` is a wrapper around it, so
    // the two agree by construction — and the sink is APPENDED to, never
    // cleared, which is the whole reason a caller uses it.
    #[test]
    fn renders_into_a_callers_sink_by_appending() {
        let mut out = "eval App.gain -> ".to_string();
        Value::F64(-0.0).render_into(&mut out);

        assert_eq!(out, "eval App.gain -> -0");
        assert_eq!(
            out,
            format!("eval App.gain -> {}", Value::F64(-0.0).render())
        );
    }

    #[test]
    fn bits_eq_compares_integers_and_bools_by_value() {
        assert!(Value::I64(5).bits_eq(&Value::I64(5)));
        assert!(!Value::I64(5).bits_eq(&Value::I64(6)));
        assert!(Value::I32(-7).bits_eq(&Value::I32(-7)));
        assert!(!Value::I32(-7).bits_eq(&Value::I32(7)));
        assert!(Value::Bool(true).bits_eq(&Value::Bool(true)));
        assert!(Value::Bool(false).bits_eq(&Value::Bool(false)));
        assert!(!Value::Bool(true).bits_eq(&Value::Bool(false)));
    }

    // The gating comparison is bitwise, so it answers where IEEE `==` gets the
    // gate wrong in both directions: a settled NaN is EQUAL, and the two zeros
    // are DIFFERENT.
    #[test]
    fn bits_eq_is_bitwise_for_floats() {
        assert!(Value::F64(f64::NAN).bits_eq(&Value::F64(f64::NAN)));
        assert!(Value::F32(f32::NAN).bits_eq(&Value::F32(f32::NAN)));
        assert!(!Value::F64(-0.0).bits_eq(&Value::F64(0.0)));
        assert!(!Value::F32(-0.0).bits_eq(&Value::F32(0.0)));
        assert!(Value::F64(1.5).bits_eq(&Value::F64(1.5)));
    }

    // Bit-pattern fidelity, not NaN-class equality, is the contract: two NaNs
    // with different payload bits are two different values here, even though
    // both render `NaN`.
    #[test]
    fn bits_eq_distinguishes_nan_payloads() {
        let quiet = f64::NAN;
        let payload = f64::from_bits(quiet.to_bits() | 1);

        assert!(payload.is_nan());
        assert_ne!(quiet.to_bits(), payload.to_bits());
        assert!(!Value::F64(quiet).bits_eq(&Value::F64(payload)));
    }

    // A value is its variant AND its bits: same digits under two widths are two
    // values, so the gate never conflates them.
    #[test]
    fn bits_eq_never_crosses_variants() {
        assert!(!Value::F64(1.0).bits_eq(&Value::F32(1.0)));
        assert!(!Value::I64(5).bits_eq(&Value::I32(5)));
        assert!(!Value::I64(1).bits_eq(&Value::Bool(true)));
    }
}
