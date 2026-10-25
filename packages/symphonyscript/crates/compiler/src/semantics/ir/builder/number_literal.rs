use symphonyscript_core::{Scalar, Vocabulary};

use crate::semantics::ir::{FloatLit, Number, Suffix};

pub(super) fn number_literal(text: &str, vocabulary: &Vocabulary) -> Option<(Number, Suffix)> {
    let bytes = text.as_bytes();
    if let Some(radix) = prefixed_radix(bytes) {
        let digits: String = text.get(2..)?.chars().filter(|c| *c != '_').collect();
        let value = u128::from_str_radix(&digits, radix).ok()?;
        let value = i128::try_from(value).ok()?;
        return Some((Number::Int(value), Suffix::None));
    }
    let mut position = 0;
    let mut digits = String::new();
    let mut fraction = 0i64;
    while let Some(&byte) = bytes.get(position) {
        if byte.is_ascii_digit() {
            digits.push(char::from(byte));
        } else if byte != b'_' {
            break;
        }
        position += 1;
    }
    let mut is_float = false;
    if bytes.get(position) == Some(&b'.') && bytes.get(position + 1).is_some_and(u8::is_ascii_digit)
    {
        is_float = true;
        position += 1;
        while let Some(&byte) = bytes.get(position) {
            if byte.is_ascii_digit() {
                digits.push(char::from(byte));
                fraction += 1;
            } else if byte != b'_' {
                break;
            }
            position += 1;
        }
    }
    let mut exponent = 0i64;
    if matches!(bytes.get(position), Some(b'e' | b'E')) {
        let signed = matches!(bytes.get(position + 1), Some(b'+' | b'-'));
        let first = if signed { position + 2 } else { position + 1 };
        if bytes.get(first).is_some_and(u8::is_ascii_digit) {
            is_float = true;
            let negative = bytes.get(position + 1) == Some(&b'-');
            position = first;
            while let Some(&byte) = bytes.get(position) {
                if byte.is_ascii_digit() {
                    exponent = exponent
                        .saturating_mul(10)
                        .saturating_add(i64::from(byte - b'0'));
                } else if byte != b'_' {
                    break;
                }
                position += 1;
            }
            if negative {
                exponent = -exponent;
            }
        }
    }
    if digits.is_empty() {
        return None;
    }
    let suffix = suffix(text.get(position..)?, vocabulary)?;
    if is_float {
        if matches!(suffix, Suffix::Type(Scalar::I32 | Scalar::I64)) {
            return None;
        }
        let float = FloatLit {
            negative: false,
            digits,
            exponent: exponent.saturating_sub(fraction),
        };
        return Some((Number::Float(float), suffix));
    }
    let value = digits.parse::<u128>().ok()?;
    let value = i128::try_from(value).ok()?;
    Some((Number::Int(value), suffix))
}

fn prefixed_radix(bytes: &[u8]) -> Option<u32> {
    let radix = match bytes.get(..2)? {
        b"0x" => 16,
        b"0b" => 2,
        b"0o" => 8,
        _ => return None,
    };
    let first = char::from(*bytes.get(2)?);
    first.is_digit(radix).then_some(radix)
}

fn suffix(text: &str, vocabulary: &Vocabulary) -> Option<Suffix> {
    let suffix = match text {
        "" => Suffix::None,
        "u8" => Suffix::Type(Scalar::U8),
        "i32" => Suffix::Type(Scalar::I32),
        "i64" => Suffix::Type(Scalar::I64),
        "f32" => Suffix::Type(Scalar::F32),
        "f64" => Suffix::Type(Scalar::F64),
        unit => {
            let (family, unit) = vocabulary.unit_named(unit)?;
            Suffix::Unit { family, unit }
        }
    };
    Some(suffix)
}
