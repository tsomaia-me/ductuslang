//! The `double_i32` stub: one `i32` in, twice it out.

use std::rc::Rc;

use ductus_runtime::provider::Behavior;
use ductus_runtime::value::Value;

/// `x -> 2 * x` over one [`Value::I32`] input.
///
/// Doubling is the smallest derived that is not the identity, which makes it
/// the default stub for witnesses whose subject is the ENGINE — dirt, order,
/// gating — rather than arithmetic: the value on an `eval` line is visibly a
/// function of the input, and a suppressed evaluation is visible as a missing
/// line.
///
/// The multiplication WRAPS. A stub is a fixture, and an overflow panic at
/// `i32::MAX` would read as a behavior trap (D8) in a witness that meant
/// nothing of the kind — so the stub stays total and the trap witness stays
/// the only place a stub aborts.
///
/// # Panics
///
/// On any other argument shape — wrong arity, or an input that is not `I32`.
/// That is a HARNESS-AUTHOR bug (a stub bound to a cell it does not fit), not
/// a runtime condition, so it fails loudly with the arguments in the message
/// instead of inventing a value.
#[must_use]
pub fn double_i32() -> Behavior {
    Rc::new(|inputs| match inputs {
        [Value::I32(payload)] => Value::I32(payload.wrapping_mul(2)),
        other => panic!("double_i32 wants one i32 input, got {other:?}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_its_input() {
        let stub = double_i32();

        assert!(stub(&[Value::I32(7)]).bits_eq(&Value::I32(14)));
        assert!(stub(&[Value::I32(-3)]).bits_eq(&Value::I32(-6)));
        assert!(stub(&[Value::I32(0)]).bits_eq(&Value::I32(0)));
    }

    // Total, not trapping: the extreme is a fixture's business, never a
    // witness's.
    #[test]
    fn wraps_instead_of_trapping_at_the_extreme() {
        let stub = double_i32();

        assert!(stub(&[Value::I32(i32::MAX)]).bits_eq(&Value::I32(-2)));
    }

    #[test]
    #[should_panic(expected = "double_i32 wants one i32 input")]
    fn panics_on_the_wrong_value_type() {
        let _ = double_i32()(&[Value::I64(7)]);
    }

    #[test]
    #[should_panic(expected = "double_i32 wants one i32 input")]
    fn panics_on_the_wrong_arity() {
        let _ = double_i32()(&[Value::I32(1), Value::I32(2)]);
    }
}
