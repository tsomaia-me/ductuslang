//! The `sum_i64` stub: every `i64` input added together.

use std::rc::Rc;

use ductus_runtime::provider::Behavior;
use ductus_runtime::value::Value;

/// `xs -> Σ xs` over any number of [`Value::I64`] inputs.
///
/// The join stub. A derived that reads TWO dependencies is what makes
/// glitch-freedom observable: if a diamond's join ever saw one new value and
/// one stale one, the sum on its `eval` line would say so. Arity is free
/// because the same stub serves a two-input join and a batch of inputs
/// committed together; the empty argument list sums to `0`.
///
/// The addition WRAPS, for the reason [`double_i32()`](super::double_i32()) gives:
/// a fixture must not trap by accident.
///
/// # Panics
///
/// If any input is not `I64` — a harness-author bug, loud rather than
/// papered over.
#[must_use]
pub fn sum_i64() -> Behavior {
    Rc::new(|inputs| {
        let mut total: i64 = 0;

        for input in inputs {
            match input {
                Value::I64(payload) => total = total.wrapping_add(*payload),
                other => panic!("sum_i64 wants i64 inputs, got {other:?} among {inputs:?}"),
            }
        }

        Value::I64(total)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_any_arity() {
        let stub = sum_i64();

        assert!(stub(&[]).bits_eq(&Value::I64(0)));
        assert!(stub(&[Value::I64(7)]).bits_eq(&Value::I64(7)));
        assert!(stub(&[Value::I64(2), Value::I64(-5)]).bits_eq(&Value::I64(-3)));
        assert!(stub(&[Value::I64(1), Value::I64(2), Value::I64(3)]).bits_eq(&Value::I64(6)));
    }

    #[test]
    fn wraps_instead_of_trapping_at_the_extreme() {
        let stub = sum_i64();

        assert!(stub(&[Value::I64(i64::MAX), Value::I64(1)]).bits_eq(&Value::I64(i64::MIN)));
    }

    #[test]
    #[should_panic(expected = "sum_i64 wants i64 inputs")]
    fn panics_on_the_wrong_value_type() {
        let _ = sum_i64()(&[Value::I64(1), Value::I32(2)]);
    }
}
