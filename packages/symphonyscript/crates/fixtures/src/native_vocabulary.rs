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
    v.build().expect("the native vocabulary builds")
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
