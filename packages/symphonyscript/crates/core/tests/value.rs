use symphonyscript_core::Value;

#[test]
fn floats_compare_by_bits() {
    assert_ne!(Value::F64(0.0), Value::F64(-0.0));
    assert_ne!(Value::F32(0.0), Value::F32(-0.0));
    assert_eq!(Value::F64(f64::NAN), Value::F64(f64::NAN));
    assert_eq!(Value::F32(f32::NAN), Value::F32(f32::NAN));
    assert_ne!(
        Value::F64(f64::from_bits(0x7FF8_0000_0000_0000)),
        Value::F64(f64::from_bits(0x7FF8_0000_0000_0001))
    );
    assert_eq!(Value::F64(1.5), Value::F64(1.5));
}

#[test]
fn values_of_different_types_differ() {
    assert_ne!(Value::I32(1), Value::I64(1));
    assert_ne!(Value::F32(1.0), Value::F64(1.0));
    assert_ne!(Value::Bool(false), Value::I32(0));
}
