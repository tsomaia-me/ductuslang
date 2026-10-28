use symphonyscript_core::{Bounded, Value};

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

#[test]
fn a_bounded_list_compares_its_elements_bit_for_bit_and_reads_at_most_one_past_its_capacity() {
    let nan = Bounded::<f32, 2>::from_items([f32::NAN]).expect("one item fits");
    assert_eq!(nan, nan.clone());
    assert_eq!(
        nan,
        Bounded::<f32, 2>::from_items([f32::from_bits(0x7FC0_0001)]).expect("one item fits")
    );
    assert_ne!(
        Bounded::<f32, 2>::from_items([-0.0]),
        Bounded::<f32, 2>::from_items([0.0])
    );
    assert_eq!(
        Bounded::<f64, 2>::from_items([1.5, 2.0]),
        Bounded::<f64, 2>::from_items([1.5, 2.0])
    );
    assert_ne!(
        Bounded::<[f32; 2], 2>::from_items([[1.0, -0.0]]),
        Bounded::<[f32; 2], 2>::from_items([[1.0, 0.0]])
    );
    assert!(Bounded::<i32, 4>::from_items(0..).is_none());
    assert!(Bounded::<i32, 4>::from_items(0..5).is_none());
    assert_eq!(
        Bounded::<i32, 4>::from_items(0..4).map(|list| list.items().to_vec()),
        Some(vec![0, 1, 2, 3])
    );
}
