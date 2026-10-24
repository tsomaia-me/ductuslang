use symphonyscript_core::{Builtin, SpecFloat};

#[test]
fn every_float_built_in_keeps_its_value_and_a_wrong_call_yields_none() {
    let rows: &[(Builtin, &[f64], Option<f64>)] = &[
        (Builtin::Abs, &[-2.5], Some(2.5)),
        (Builtin::Min, &[1.0, 2.0], Some(1.0)),
        (Builtin::Min, &[f64::NAN, 2.0], Some(2.0)),
        (Builtin::Max, &[1.0, 2.0], Some(2.0)),
        (Builtin::Max, &[1.0, f64::NAN], Some(1.0)),
        (Builtin::Clamp, &[5.0, 0.0, 3.0], Some(3.0)),
        (Builtin::Clamp, &[-1.0, 0.0, 3.0], Some(0.0)),
        (Builtin::Clamp, &[1.0, 4.0, 3.0], Some(3.0)),
        (Builtin::Floor, &[-1.5], Some(-2.0)),
        (Builtin::Ceil, &[-1.5], Some(-1.0)),
        (Builtin::Round, &[2.5], Some(3.0)),
        (Builtin::Round, &[-2.5], Some(-3.0)),
        (Builtin::Sqrt, &[4.0], Some(2.0)),
        (Builtin::Sin, &[0.0], Some(0.0)),
        (Builtin::Cos, &[0.0], Some(1.0)),
        (Builtin::Tan, &[0.0], Some(0.0)),
        (Builtin::Exp, &[0.0], Some(1.0)),
        (Builtin::Log, &[1.0], Some(0.0)),
        (Builtin::Lerp, &[1.0, 3.0, 0.5], Some(2.0)),
        (Builtin::Abs, &[], None),
        (Builtin::Abs, &[1.0, 2.0], None),
        (Builtin::Min, &[1.0], None),
        (Builtin::Clamp, &[1.0, 2.0], None),
        (Builtin::Lerp, &[1.0, 2.0], None),
        (Builtin::Rand, &[0.0, 0.0], None),
    ];
    for (builtin, args, expected) in rows {
        assert_eq!(
            f64::spec_builtin(*builtin, args).map(f64::to_bits),
            expected.map(f64::to_bits),
            "{builtin:?} {args:?} in f64"
        );
        let narrow: Vec<f32> = args.iter().map(|arg| *arg as f32).collect();
        assert_eq!(
            f32::spec_builtin(*builtin, &narrow).map(f32::to_bits),
            expected.map(|value| (value as f32).to_bits()),
            "{builtin:?} {args:?} in f32"
        );
    }
    assert_eq!(
        f64::spec_builtin(Builtin::Min, &[-0.0, 0.0]).map(f64::to_bits),
        Some((-0.0f64).to_bits())
    );
    assert_eq!(
        f64::spec_builtin(Builtin::Max, &[-0.0, 0.0]).map(f64::to_bits),
        Some(0.0f64.to_bits())
    );
}
