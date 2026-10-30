use symphonyscript_core::{
    Bounded, Count, NativeArg, NativeOut, NativeSignature, NativeType, Ty, Value, Vocabulary,
};

pub fn signature_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("signatures");
    let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
    v.enumeration("Wave", ["Sine", "Saw", "Square"]);
    let clip = v.node("Clip");
    let note = v.node("Note");
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any)
        .attr::<[u8; 3]>("fixed")
        .default([0, 0, 0])
        .attr::<Bounded<u8, 8>>("data")
        .default(Bounded::default())
        .attr::<Bounded<[i32; 2], 2>>("rows")
        .default(Bounded::default())
        .attr::<Bounded<i32, 3>>("three")
        .default(Bounded::default());
    v.kind(note)
        .attr::<i32>("pitch")
        .default(60)
        .default_attr()
        .attr::<f64>("length")
        .default(0.25)
        .family(&beats, "beats")
        .attr::<u8>("level")
        .default(0);
    v.native(
        "plain",
        NativeSignature::new([NativeType::Value(Ty::I32)], NativeType::Value(Ty::I32)),
        first,
    );
    v.native(
        "half",
        NativeSignature::new([NativeType::Value(Ty::F64)], NativeType::Value(Ty::F64)),
        half,
    );
    v.native(
        "cap4",
        NativeSignature::new(
            [NativeType::Value(Ty::I32.variable(4))],
            NativeType::Value(Ty::I32),
        ),
        first,
    );
    v.native(
        "fixed3",
        NativeSignature::new(
            [NativeType::Value(Ty::U8.array(3))],
            NativeType::Value(Ty::I32),
        ),
        first,
    );
    v.native(
        "shrink",
        NativeSignature::new(
            [NativeType::Open(Ty::I32.array(3))],
            NativeType::CapacityOf(Ty::I32.array(2), 0),
        ),
        shrink,
    );
    v.native(
        "squeeze",
        NativeSignature::new(
            [NativeType::Value(Ty::I32.variable(4))],
            NativeType::CapacityOf(Ty::I32, 0),
        ),
        shrink,
    );
    v.native(
        "label",
        NativeSignature::new([NativeType::Value(Ty::I32)], NativeType::Value(Ty::STR)),
        label,
    );
    v.build().expect("the signature vocabulary builds")
}

fn first(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let value = match args.first() {
        Some(NativeArg::Value(value)) => Some(*value),
        Some(NativeArg::Elements(items)) => items.first().copied(),
        Some(NativeArg::Text(_)) | None => None,
    };
    out.set(match value {
        Some(Value::U8(byte)) => Value::I32(i32::from(byte)),
        Some(value) => value,
        None => Value::I32(0),
    });
}

fn half(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let value = match args.first() {
        Some(NativeArg::Value(Value::F64(value))) => *value,
        Some(_) | None => 0.0,
    };
    out.set(Value::F64(value / 2.0));
}

fn shrink(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    if let Some(NativeArg::Elements(items)) = args.first() {
        for item in *items {
            if !out.push(*item) {
                break;
            }
        }
    }
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
