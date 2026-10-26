use symphonyscript_core::{
    Count, NativeArg, NativeFn, NativeOut, NativeSignature, NativeType, Ty, Value, Vocabulary,
    VocabularyStamp,
};

struct Options {
    doc: bool,
    children_reversed: bool,
    velocity: f32,
    prelude: &'static str,
    prelude_path: &'static str,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            doc: false,
            children_reversed: false,
            velocity: 1.0,
            prelude: "let a = 1\nlet b = 2\n",
            prelude_path: "music.sym",
        }
    }
}

fn stamp(options: Options) -> VocabularyStamp {
    let mut v = Vocabulary::builder("music");
    let start = v.unit_family("beats");
    let start = if options.doc {
        start.doc("Musical time.")
    } else {
        start
    };
    let beats = start.base("beats").unit("bars", 4.0);
    let beats = if options.doc {
        beats.doc("Four beats.")
    } else {
        beats
    };
    let wave = v.enumeration("Wave", ["Sine", "Saw"]);
    let wave = if options.doc {
        wave.doc("A waveform.")
    } else {
        wave
    };
    let note = v.node("Note");
    let rest = v.node("Rest");
    let clip = v.node("Clip");
    let plays = v.wire("Plays");
    let kind = v.kind(note);
    let kind = if options.doc {
        kind.doc("A note.")
    } else {
        kind
    };
    let attr = kind.attr::<f32>("velocity").default(options.velocity);
    let attr = if options.doc {
        attr.doc("Loudness.")
    } else {
        attr
    };
    attr.attr::<f64>("length")
        .default(0.25)
        .family(&beats, "beats")
        .attr_enum("wave", &wave, "Saw");
    let clip_kind = v.kind(clip).top_level(Count::Any);
    if options.children_reversed {
        clip_kind
            .children(rest, Count::AtMost(2))
            .children(note, Count::Any);
    } else {
        clip_kind
            .children(note, Count::Any)
            .children(rest, Count::AtMost(2));
    }
    let wire = v.wire_kind(plays).connects(clip, clip).cyclic();
    if options.doc {
        wire.doc("Plays another clip.");
    }
    v.prelude(options.prelude_path, options.prelude);
    v.build().expect("the vocabulary is valid").stamp().clone()
}

#[test]
fn the_stamp_is_identical_across_builds() {
    assert_eq!(stamp(Options::default()), stamp(Options::default()));
    assert_eq!(stamp(Options::default()).name, "music");
}

#[test]
fn documentation_does_not_change_the_stamp() {
    assert_eq!(
        stamp(Options::default()),
        stamp(Options {
            doc: true,
            ..Options::default()
        })
    );
}

#[test]
fn the_order_of_child_declarations_does_not_change_the_stamp() {
    assert_eq!(
        stamp(Options::default()),
        stamp(Options {
            children_reversed: true,
            ..Options::default()
        })
    );
}

#[test]
fn prelude_line_endings_and_path_do_not_change_the_stamp() {
    assert_eq!(
        stamp(Options::default()),
        stamp(Options {
            prelude: "let a = 1\r\nlet b = 2\r\n",
            prelude_path: "elsewhere.sym",
            ..Options::default()
        })
    );
}

#[test]
fn content_changes_change_the_stamp() {
    let base = stamp(Options::default());
    assert_ne!(
        base,
        stamp(Options {
            velocity: 0.5,
            ..Options::default()
        })
    );
    assert_ne!(
        base,
        stamp(Options {
            prelude: "let a = 1\nlet b = 3\n",
            ..Options::default()
        })
    );
}

#[test]
fn nested_and_enum_array_attributes_keep_their_pinned_stamp() {
    let mut v = Vocabulary::builder("arrays");
    let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);
    let grid = v.node("Grid");
    v.kind(grid)
        .top_level(Count::Any)
        .attr::<[[i32; 3]; 2]>("cells")
        .default([[1, 2, 3], [4, 5, 6]])
        .attr_enum_array("waves", &wave, 4, "Saw")
        .attr::<[[[bool; 1]; 2]; 3]>("gates")
        .default([[[true], [false]], [[false], [true]], [[true], [true]]]);
    let stamp = v.build().expect("the vocabulary is valid").stamp().clone();
    assert_eq!(stamp.name, "arrays");
    assert_eq!(stamp.hash, 0x35f8_efc8_552d_30a7);
}

fn one(_: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    out.set(Value::I32(1));
}

fn two(_: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    out.set(Value::I32(2));
}

fn native_stamp(name: &str, param: Ty, function: NativeFn) -> VocabularyStamp {
    let mut v = Vocabulary::builder("natives");
    v.node("Note");
    v.native(
        name,
        NativeSignature::new([NativeType::Value(param)], NativeType::Value(Ty::I32)),
        function,
    );
    v.build().expect("the vocabulary builds").stamp().clone()
}

#[test]
fn a_natives_name_and_signature_change_the_stamp_and_its_function_does_not() {
    let base = native_stamp("f", Ty::I32, one);
    assert_eq!(native_stamp("f", Ty::I32, one), base);
    assert_eq!(native_stamp("f", Ty::I32, two), base);
    assert_ne!(native_stamp("g", Ty::I32, one), base);
    assert_ne!(native_stamp("f", Ty::I64, one), base);
}
