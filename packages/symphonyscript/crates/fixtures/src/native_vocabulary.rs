use symphonyscript_core::{
    Bounded, Count, NativeArg, NativeOut, NativeSignature, NativeType, Ty, Value, Vocabulary,
};

pub fn native_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("natives");
    let song = v.node("Song");
    let clip = v.node("Clip");
    let note = v.node("Note");
    v.kind(song)
        .top_level(Count::AtMost(1))
        .entry()
        .children(clip, Count::Any);
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any)
        .attr::<Bounded<i32, 8>>("sequence")
        .default(Bounded::default())
        .attr::<f32>("gain")
        .default(1.0)
        .attr::<Bounded<i32, 4>>("seq")
        .default(Bounded::from_items([7, 8]).unwrap_or_default())
        .attr::<[i32; 3]>("trio")
        .default([0, 0, 0])
        .attr::<Bounded<[i32; 2], 3>>("rows")
        .default(Bounded::from_items([[1, 2], [3, 4]]).unwrap_or_default());
    v.kind(note).attr::<i32>("pitch").default(60).default_attr();
    v.native(
        "checksum",
        NativeSignature::new([NativeType::Open(Ty::U8)], NativeType::Value(Ty::I32)),
        checksum,
    );
    v.native(
        "text_length",
        NativeSignature::new([NativeType::Value(Ty::STR)], NativeType::Value(Ty::I32)),
        text_length,
    );
    v.native(
        "mirror",
        NativeSignature::new(
            [NativeType::Open(Ty::I32)],
            NativeType::CapacityOf(Ty::I32, 0),
        ),
        mirror,
    );
    v.native(
        "pair_sum",
        NativeSignature::new(
            [NativeType::Value(Ty::I32.array(2))],
            NativeType::Value(Ty::I32),
        ),
        pair_sum,
    );
    v.native(
        "first_of",
        NativeSignature::new(
            [NativeType::Value(Ty::I32.variable(4))],
            NativeType::Value(Ty::I32),
        ),
        first_of,
    );
    v.native(
        "tag",
        NativeSignature::new(
            [NativeType::Value(Ty::STR), NativeType::Value(Ty::I32)],
            NativeType::Value(Ty::I32),
        ),
        tag,
    );
    v.native(
        "label",
        NativeSignature::new([NativeType::Value(Ty::I32)], NativeType::Value(Ty::STR)),
        label,
    );
    for (name, result, run) in [
        ("three", NativeType::Value(Ty::I32.array(3)), counted as Run),
        ("halves", NativeType::Value(Ty::F32.array(2)), halves),
        ("upto", NativeType::Value(Ty::I32.variable(5)), counted),
        (
            "grid",
            NativeType::Value(Ty::I32.array(2).array(3)),
            counted,
        ),
        ("octets", NativeType::Value(Ty::U8.variable(4)), octets),
    ] {
        v.native(
            name,
            NativeSignature::new([NativeType::Value(Ty::I32)], result),
            run,
        );
    }
    v.build().expect("the native vocabulary builds")
}

type Run = fn(&[NativeArg<'_>], &mut NativeOut<'_>);

fn first_i32(args: &[NativeArg<'_>]) -> i32 {
    match args.first() {
        Some(NativeArg::Value(Value::I32(value))) => *value,
        Some(NativeArg::Value(_) | NativeArg::Elements(_) | NativeArg::Text(_)) | None => 0,
    }
}

fn counted(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let mut next = first_i32(args);
    while out.push(Value::I32(next)) {
        next = next.saturating_add(1);
    }
}

fn halves(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let value = f32::from(i16::try_from(first_i32(args)).unwrap_or(0)) / 2.0;
    while out.push(Value::F32(value)) {}
}

fn octets(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let length = usize::try_from(first_i32(args).clamp(0, 4)).unwrap_or(0);
    for byte in 0..length {
        out.push(Value::U8(u8::try_from(byte).unwrap_or(u8::MAX)));
    }
}

fn checksum(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let mut sum = 0i32;
    if let Some(NativeArg::Elements(bytes)) = args.first() {
        for byte in *bytes {
            if let Value::U8(byte) = byte {
                sum = sum.wrapping_add(i32::from(*byte));
            }
        }
    }
    out.set(Value::I32(sum));
}

fn text_length(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let length = match args.first() {
        Some(NativeArg::Text(text)) => i32::try_from(text.chars().count()).unwrap_or(i32::MAX),
        Some(NativeArg::Value(_) | NativeArg::Elements(_)) | None => 0,
    };
    out.set(Value::I32(length));
}

fn mirror(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    if let Some(NativeArg::Elements(items)) = args.first() {
        for item in items.iter().rev() {
            out.push(*item);
        }
    }
}

fn pair_sum(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let mut sum = 0i32;
    if let Some(NativeArg::Elements(items)) = args.first() {
        for item in *items {
            if let Value::I32(value) = item {
                sum = sum.saturating_add(*value);
            }
        }
    }
    out.set(Value::I32(sum));
}

fn first_of(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let first = match args.first() {
        Some(NativeArg::Elements(items)) => items.first().copied(),
        Some(NativeArg::Value(_) | NativeArg::Text(_)) | None => None,
    };
    out.set(first.unwrap_or(Value::I32(0)));
}

fn tag(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let length = match args.first() {
        Some(NativeArg::Text(text)) => i32::try_from(text.chars().count()).unwrap_or(i32::MAX),
        Some(NativeArg::Value(_) | NativeArg::Elements(_)) | None => 0,
    };
    let offset = match args.get(1) {
        Some(NativeArg::Value(Value::I32(offset))) => *offset,
        Some(NativeArg::Value(_) | NativeArg::Elements(_) | NativeArg::Text(_)) | None => 0,
    };
    out.set(Value::I32(length.saturating_add(offset)));
}

fn label(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let text = match args.first() {
        Some(NativeArg::Value(Value::I32(number))) => number.to_string(),
        Some(NativeArg::Value(_) | NativeArg::Elements(_) | NativeArg::Text(_)) | None => {
            String::new()
        }
    };
    out.set_text(&text);
}
