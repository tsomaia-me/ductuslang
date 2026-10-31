use symphonyscript_core::{
    Bounded, Count, NativeArg, NativeOut, NativeSignature, NativeType, Ty, Vocabulary,
    VocabularyBuilder, VocabularyError,
};

fn errors(builder: VocabularyBuilder) -> Vec<VocabularyError> {
    match builder.build() {
        Ok(_) => Vec::new(),
        Err(invalid) => invalid.errors,
    }
}

fn single(builder: VocabularyBuilder, expected: VocabularyError) {
    assert_eq!(errors(builder), vec![expected]);
}

fn s(text: &str) -> String {
    String::from(text)
}

#[test]
fn a_valid_vocabulary_has_no_errors() {
    let mut v = Vocabulary::builder("v");
    let note = v.node("Note");
    v.kind(note).attr::<i32>("pitch").default(60);
    assert_eq!(errors(v), vec![]);
}

#[test]
fn names_must_be_ascii_identifiers() {
    for name in ["Nöte", "1Note", "Note-x", "_", "#", "__#", ""] {
        let mut v = Vocabulary::builder("v");
        v.node(name);
        single(v, VocabularyError::NotAsciiIdentifier { name: s(name) });
    }
    let mut v = Vocabulary::builder("v");
    v.node("C#4");
    v.node("_x");
    v.node("#a");
    assert_eq!(errors(v), vec![]);
}

#[test]
fn names_must_not_be_keywords_or_reserved_words_but_an_attribute_may_be_a_keyword() {
    for name in [
        "state",
        "channel",
        "match",
        "bool",
        "into",
        "from",
        "recurrent",
        "string",
    ] {
        let mut v = Vocabulary::builder("v");
        v.node(name);
        single(v, VocabularyError::ReservedName { name: s(name) });
    }
    for name in ["recurrent", "string"] {
        let mut v = Vocabulary::builder("v");
        let note = v.node("Note");
        v.kind(note).attr::<i32>(name);
        single(v, VocabularyError::ReservedName { name: s(name) });
    }
    for name in ["channel", "out", "on", "match", "state"] {
        let mut v = Vocabulary::builder("v");
        let note = v.node("Note");
        v.kind(note).attr::<i32>(name);
        assert_eq!(errors(v), vec![], "{name}");
    }
}

#[test]
fn names_must_not_be_built_ins() {
    let mut v = Vocabulary::builder("v");
    v.wire("pi");
    single(v, VocabularyError::BuiltinName { name: s("pi") });
}

#[test]
fn names_must_not_be_variants() {
    let mut v = Vocabulary::builder("v");
    v.enumeration("Wave", ["Sine", "Saw"]);
    let osc = v.node("Saw");
    v.kind(osc).attr::<f32>("Sine");
    v.unit_family("Sine").base("Saw");
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::VariantName { name: s("Saw") },
            VocabularyError::VariantName { name: s("Sine") },
            VocabularyError::VariantName { name: s("Sine") },
            VocabularyError::VariantName { name: s("Saw") },
        ]
    );
}

#[test]
fn variants_must_be_identifiers_and_not_keywords_or_built_ins() {
    let mut v = Vocabulary::builder("v");
    v.enumeration("Wave", ["Sine", "is", "abs", "Sä"]);
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::ReservedName { name: s("is") },
            VocabularyError::BuiltinName { name: s("abs") },
            VocabularyError::NotAsciiIdentifier { name: s("Sä") },
        ]
    );
}

#[test]
fn two_declarations_must_not_share_a_name() {
    let mut v = Vocabulary::builder("v");
    v.node("Note");
    v.wire("Note");
    v.enumeration("Wave", ["Sine"]);
    v.unit_family("Wave").base("s");
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::DuplicateName { name: s("Note") },
            VocabularyError::DuplicateName { name: s("Wave") },
        ]
    );
}

#[test]
fn a_kind_must_not_declare_two_attributes_of_one_name() {
    let mut v = Vocabulary::builder("v");
    let plays = v.wire("Plays");
    v.wire_kind(plays).attr::<f32>("gain").attr::<i32>("gain");
    single(
        v,
        VocabularyError::DuplicateAttribute {
            owner: s("Plays"),
            name: s("gain"),
        },
    );
}

#[test]
fn a_kind_must_not_have_two_default_attributes() {
    let mut v = Vocabulary::builder("v");
    let note = v.node("Note");
    v.kind(note)
        .attr::<i32>("pitch")
        .default_attr()
        .attr::<f32>("velocity")
        .default_attr();
    single(
        v,
        VocabularyError::DuplicateDefaultAttribute { owner: s("Note") },
    );
}

#[test]
fn two_attributes_must_not_share_a_flag() {
    let mut v = Vocabulary::builder("v");
    let note = v.node("Note");
    v.kind(note)
        .attr::<bool>("accent")
        .flag('!')
        .attr::<bool>("mute")
        .flag('!');
    single(
        v,
        VocabularyError::DuplicateFlag {
            owner: s("Note"),
            flag: '!',
        },
    );
}

#[test]
fn a_flag_must_be_a_flag_character() {
    let mut v = Vocabulary::builder("v");
    let note = v.node("Note");
    v.kind(note).attr::<bool>("accent").flag('%');
    single(
        v,
        VocabularyError::InvalidFlag {
            owner: s("Note"),
            attribute: s("accent"),
            flag: '%',
        },
    );
}

#[test]
fn a_flag_must_flag_a_bool_attribute() {
    let mut v = Vocabulary::builder("v");
    let wave = v.enumeration("Wave", ["Sine"]);
    let note = v.node("Note");
    v.kind(note)
        .attr::<i32>("pitch")
        .flag('*')
        .attr::<[bool; 2]>("steps")
        .flag('?')
        .attr_enum("wave", wave, "Sine")
        .flag('~');
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::FlagOnNonBool {
                owner: s("Note"),
                attribute: s("pitch"),
            },
            VocabularyError::FlagOnNonBool {
                owner: s("Note"),
                attribute: s("steps"),
            },
            VocabularyError::FlagOnNonBool {
                owner: s("Note"),
                attribute: s("wave"),
            },
        ]
    );
}

#[test]
fn every_flag_character_is_accepted() {
    let mut v = Vocabulary::builder("v");
    let note = v.node("Note");
    ['!', '?', '*', '+', '~', '@', '$']
        .into_iter()
        .enumerate()
        .fold(
            v.kind(note).attr::<bool>("a0").flag('\''),
            |kind, (position, flag)| kind.attr::<bool>(&format!("a{}", position + 1)).flag(flag),
        );
    assert_eq!(errors(v), vec![]);
}

#[test]
fn a_count_minimum_must_not_exceed_its_maximum() {
    let mut v = Vocabulary::builder("v");
    let note = v.node("Note");
    let clip = v.node("Clip");
    let plays = v.wire("Plays");
    v.kind(clip)
        .top_level(Count::Between(3, 2))
        .children(note, Count::Between(5, 1))
        .incoming(plays, Count::Between(2, 2))
        .outgoing(plays, Count::Between(1, 0));
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::CountRange {
                owner: s("Clip"),
                count: Count::Between(3, 2),
            },
            VocabularyError::CountRange {
                owner: s("Clip"),
                count: Count::Between(5, 1),
            },
            VocabularyError::CountRange {
                owner: s("Clip"),
                count: Count::Between(1, 0),
            },
        ]
    );
}

#[test]
fn a_declaration_must_not_refer_to_another_vocabulary() {
    let mut other = Vocabulary::builder("other");
    let foreign_node = other.node("Foreign");
    let foreign_wire = other.wire("Link");
    let foreign_enum = other.enumeration("Mode", ["On"]);
    let foreign_family = other.unit_family("time").base("s");

    let mut v = Vocabulary::builder("v");
    let clip = v.node("Clip");
    let plays = v.wire("Plays");
    v.kind(clip)
        .children(foreign_node, Count::Any)
        .incoming(foreign_wire, Count::Any)
        .attr_enum("mode", foreign_enum, "On")
        .attr::<f64>("length")
        .family(foreign_family, "s");
    v.wire_kind(plays).connects(foreign_node, clip);
    v.kind(foreign_node).top_level(Count::Any);
    v.wire_kind(foreign_wire).cyclic();
    let referrer = |name: &str| VocabularyError::ForeignHandle {
        referrer: Some(s(name)),
    };
    assert_eq!(
        errors(v),
        vec![
            referrer("Clip"),
            referrer("Clip"),
            referrer("Clip"),
            referrer("Clip"),
            referrer("Plays"),
            VocabularyError::ForeignHandle { referrer: None },
            VocabularyError::ForeignHandle { referrer: None },
        ]
    );
}

#[test]
fn an_enum_must_have_between_one_and_65536_variants() {
    let mut v = Vocabulary::builder("v");
    v.enumeration("Empty", Vec::<String>::new());
    v.enumeration("Huge", (0..65_537).map(|n| format!("V{n}")));
    v.enumeration("Largest", (0..65_536).map(|n| format!("W{n}")));
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::EmptyEnum { name: s("Empty") },
            VocabularyError::EnumTooLarge {
                name: s("Huge"),
                variants: 65_537,
            },
        ]
    );
}

#[test]
fn an_enum_default_must_be_a_variant() {
    let mut v = Vocabulary::builder("v");
    let wave = v.enumeration("Wave", ["Sine", "Saw"]);
    let osc = v.node("Osc");
    v.kind(osc)
        .attr_enum("wave", &wave, "Triangle")
        .attr_enum_array("waves", &wave, 4, "Square");
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::UnknownDefaultVariant {
                owner: s("Osc"),
                attribute: s("wave"),
                variant: s("Triangle"),
            },
            VocabularyError::UnknownDefaultVariant {
                owner: s("Osc"),
                attribute: s("waves"),
                variant: s("Square"),
            },
        ]
    );
}

#[test]
fn a_unit_scale_must_be_positive_and_finite() {
    let mut v = Vocabulary::builder("v");
    v.unit_family("time")
        .base("s")
        .unit("ms", 0.001)
        .unit("zero", 0.0)
        .unit("neg", -1.0)
        .unit("inf", f64::INFINITY)
        .unit("nan", f64::NAN);
    let found = errors(v);
    let units: Vec<&str> = found
        .iter()
        .map(|error| match error {
            VocabularyError::InvalidScale { unit, .. } => unit.as_str(),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(units, ["zero", "neg", "inf", "nan"]);
}

#[test]
fn a_unit_name_must_not_repeat() {
    let mut v = Vocabulary::builder("v");
    v.unit_family("time").base("s").unit("ms", 0.001);
    v.unit_family("beats")
        .base("beats")
        .unit("ms", 2.0)
        .unit("beats", 1.0);
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::RepeatedUnit { unit: s("ms") },
            VocabularyError::RepeatedUnit { unit: s("beats") },
        ]
    );
}

#[test]
fn a_default_unit_must_be_in_its_family() {
    let mut v = Vocabulary::builder("v");
    let time = v.unit_family("time").base("s").unit("ms", 0.001);
    v.unit_family("beats").base("beats");
    let note = v.node("Note");
    v.kind(note).attr::<f64>("length").family(time, "beats");
    single(
        v,
        VocabularyError::UnknownDefaultUnit {
            owner: s("Note"),
            attribute: s("length"),
            unit: s("beats"),
        },
    );
}

#[test]
fn a_unit_name_must_not_read_as_a_number() {
    for unit in ["xa", "x9", "b0", "b1", "o7", "e1", "E5", "_ms", "__x", "_1"] {
        let mut v = Vocabulary::builder("v");
        v.unit_family("f").base("u").unit(unit, 2.0);
        single(v, VocabularyError::NumericUnitName { unit: s(unit) });
    }
    let mut v = Vocabulary::builder("v");
    v.unit_family("f")
        .base("u")
        .unit("xg", 2.0)
        .unit("b2", 3.0)
        .unit("o8", 4.0)
        .unit("e", 5.0)
        .unit("eighths", 0.5)
        .unit("bars", 4.0)
        .unit("x", 6.0);
    assert_eq!(errors(v), vec![]);
}

#[test]
fn an_integer_attribute_must_not_belong_to_a_family_with_a_fractional_scale() {
    let mut v = Vocabulary::builder("v");
    let time = v.unit_family("time").base("s").unit("ms", 0.001);
    let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
    let note = v.node("Note");
    v.kind(note)
        .attr::<i32>("delay")
        .family(&time, "s")
        .attr::<[i64; 2]>("delays")
        .family(&time, "ms")
        .attr::<i32>("length")
        .family(&beats, "beats")
        .attr::<f32>("offset")
        .family(&time, "ms");
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::FractionalScale {
                owner: s("Note"),
                attribute: s("delay"),
                unit: s("ms"),
            },
            VocabularyError::FractionalScale {
                owner: s("Note"),
                attribute: s("delays"),
                unit: s("ms"),
            },
        ]
    );
}

#[test]
fn every_error_is_reported_at_once() {
    let mut v = Vocabulary::builder("v");
    let note = v.node("if");
    v.kind(note).attr::<i32>("pi").flag('!');
    v.enumeration("Wave", Vec::<&str>::new());
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::ReservedName { name: s("if") },
            VocabularyError::BuiltinName { name: s("pi") },
            VocabularyError::FlagOnNonBool {
                owner: s("if"),
                attribute: s("pi"),
            },
            VocabularyError::EmptyEnum { name: s("Wave") },
        ]
    );
}

#[test]
fn errors_display_as_plain_messages() {
    let mut v = Vocabulary::builder("v");
    v.node("Note");
    v.node("Note");
    let invalid = v.build().expect_err("the vocabulary is invalid");
    assert_eq!(
        invalid.to_string(),
        "the vocabulary is invalid\ntwo declarations are named `Note`"
    );
}

#[test]
fn a_node_kind_must_not_declare_two_attributes_of_one_name() {
    let mut v = Vocabulary::builder("v");
    let note = v.node("Note");
    v.kind(note).attr::<i32>("pitch").attr::<bool>("pitch");
    single(
        v,
        VocabularyError::DuplicateAttribute {
            owner: s("Note"),
            name: s("pitch"),
        },
    );
}

#[test]
fn an_enum_must_not_declare_two_variants_of_one_name() {
    let mut v = Vocabulary::builder("v");
    v.enumeration("Wave", ["Sine", "Saw", "Sine", "Saw", "Sine"]);
    v.enumeration("Shape", ["Sine", "Round"]);
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::DuplicateVariant {
                enumeration: s("Wave"),
                variant: s("Sine"),
            },
            VocabularyError::DuplicateVariant {
                enumeration: s("Wave"),
                variant: s("Saw"),
            },
            VocabularyError::DuplicateVariant {
                enumeration: s("Wave"),
                variant: s("Sine"),
            },
        ]
    );
}

#[test]
fn a_unit_name_must_not_repeat_within_a_family() {
    let mut v = Vocabulary::builder("v");
    v.unit_family("time")
        .base("s")
        .unit("ms", 0.001)
        .unit("ms", 0.002)
        .unit("s", 2.0);
    assert_eq!(
        errors(v),
        vec![
            VocabularyError::RepeatedUnit { unit: s("ms") },
            VocabularyError::RepeatedUnit { unit: s("s") },
        ]
    );
}

#[test]
fn an_array_attribute_must_not_have_a_length_of_zero() {
    let mut v = Vocabulary::builder("v");
    let wave = v.enumeration("Wave", ["Sine"]);
    let clip = v.node("Clip");
    let plays = v.wire("Plays");
    v.kind(clip)
        .attr::<[i32; 0]>("empty")
        .attr::<[[bool; 0]; 2]>("inner")
        .attr::<[[f32; 3]; 0]>("outer")
        .attr_enum_array("waves", &wave, 0, "Sine")
        .attr::<[[i64; 2]; 3]>("full");
    v.wire_kind(plays).attr::<[f64; 0]>("steps");
    let empty = |owner: &str, attribute: &str| VocabularyError::EmptyArrayAttribute {
        owner: s(owner),
        attribute: s(attribute),
    };
    assert_eq!(
        errors(v),
        vec![
            empty("Clip", "empty"),
            empty("Clip", "inner"),
            empty("Clip", "outer"),
            empty("Clip", "waves"),
            empty("Plays", "steps"),
        ]
    );
}

fn nothing(_: &[NativeArg<'_>], _: &mut NativeOut<'_>) {}

#[test]
fn a_variable_array_attribute_must_not_have_a_capacity_of_zero() {
    let mut v = Vocabulary::builder("v");
    let clip = v.node("Clip");
    v.kind(clip)
        .attr::<Bounded<i32, 0>>("none")
        .attr::<Bounded<i32, 1>>("one");
    single(
        v,
        VocabularyError::EmptyArrayAttribute {
            owner: s("Clip"),
            attribute: s("none"),
        },
    );
}

#[test]
fn an_array_attribute_must_have_a_length_and_capacity_below_two_to_the_thirty_one() {
    let mut v = Vocabulary::builder("v");
    let clip = v.node("Clip");
    let plays = v.wire("Plays");
    v.kind(clip)
        .attr::<Bounded<i32, 2147483647>>("widest")
        .attr::<Bounded<i32, 2147483648>>("wide")
        .attr::<Bounded<i32, 4294967296>>("wider")
        .attr::<[i32; 2147483647]>("longest")
        .attr::<[i32; 2147483648]>("long")
        .attr::<[i32; 4294967296]>("longer")
        .attr::<Bounded<[i32; 2147483648], 1>>("inner");
    v.wire_kind(plays).attr::<Bounded<f64, 2147483648>>("steps");
    let long = |owner: &str, attribute: &str| VocabularyError::LongArrayAttribute {
        owner: s(owner),
        attribute: s(attribute),
    };
    assert_eq!(
        errors(v),
        vec![
            long("Clip", "wide"),
            long("Clip", "wider"),
            long("Clip", "long"),
            long("Clip", "longer"),
            long("Clip", "inner"),
            long("Plays", "steps"),
        ]
    );
}

#[test]
fn a_native_parameter_or_result_must_have_a_capacity_below_two_to_the_thirty_one() {
    let long = |native: &str| VocabularyError::LongNativeArray { native: s(native) };
    let rows: Vec<(NativeSignature, Vec<VocabularyError>)> = vec![
        (
            NativeSignature::new(
                [NativeType::Value(Ty::I32.variable(2147483647))],
                NativeType::Value(Ty::I32.variable(2147483647)),
            ),
            vec![],
        ),
        (
            NativeSignature::new(
                [NativeType::Value(Ty::I32.variable(2147483648))],
                NativeType::Value(Ty::I32),
            ),
            vec![long("f")],
        ),
        (
            NativeSignature::new(
                [NativeType::Value(Ty::I32)],
                NativeType::Value(Ty::I32.variable(2147483648)),
            ),
            vec![long("f")],
        ),
        (
            NativeSignature::new(
                [NativeType::Value(Ty::I32.array(2147483648))],
                NativeType::Value(Ty::I32),
            ),
            vec![long("f")],
        ),
        (
            NativeSignature::new(
                [NativeType::Open(Ty::I32.array(2147483648))],
                NativeType::CapacityOf(Ty::I32, 0),
            ),
            vec![long("f")],
        ),
    ];
    for (signature, expected) in rows {
        let mut v = Vocabulary::builder("v");
        v.native("f", signature.clone(), nothing);
        assert_eq!(errors(v), expected, "{signature:?}");
    }
    let message = VocabularyError::LongNativeArray { native: s("f") }.to_string();
    assert!(message.contains("2³¹"), "{message}");
}

#[test]
fn a_native_signature_holds_types_of_section_3_and_states_a_variable_results_capacity() {
    let outside = |native: &str| VocabularyError::NativeTypeOutsideSpec { native: s(native) };
    let unstated = |native: &str| VocabularyError::NativeCapacityUnstated { native: s(native) };
    let rows: Vec<(NativeSignature, Vec<VocabularyError>)> = vec![
        (
            NativeSignature::new(
                [NativeType::Open(Ty::I32)],
                NativeType::CapacityOf(Ty::I32, 0),
            ),
            vec![],
        ),
        (
            NativeSignature::new(
                [NativeType::Value(Ty::STR.array(2))],
                NativeType::Value(Ty::I32),
            ),
            vec![outside("f")],
        ),
        (
            NativeSignature::new(
                [NativeType::Value(Ty::I32.array(0))],
                NativeType::Value(Ty::I32),
            ),
            vec![outside("f")],
        ),
        (
            NativeSignature::new([NativeType::Value(Ty::I32)], NativeType::Open(Ty::STR)),
            vec![outside("f")],
        ),
        (
            NativeSignature::new([NativeType::Open(Ty::I32)], NativeType::Open(Ty::I32)),
            vec![unstated("f")],
        ),
        (
            NativeSignature::new(
                [NativeType::Open(Ty::I32)],
                NativeType::CapacityOf(Ty::I32, 1),
            ),
            vec![unstated("f")],
        ),
        (
            NativeSignature::new(
                [NativeType::Value(Ty::I32)],
                NativeType::CapacityOf(Ty::I32, 0),
            ),
            vec![unstated("f")],
        ),
    ];
    for (signature, expected) in rows {
        let mut v = Vocabulary::builder("v");
        v.native("f", signature.clone(), nothing);
        assert_eq!(errors(v), expected, "{signature:?}");
    }
}

#[test]
fn a_native_functions_name_is_checked_as_every_declared_name_is() {
    let signature = NativeSignature::new([NativeType::Value(Ty::I32)], NativeType::Value(Ty::I32));
    let rows: Vec<(&str, VocabularyError)> = vec![
        ("abs", VocabularyError::BuiltinName { name: s("abs") }),
        ("match", VocabularyError::ReservedName { name: s("match") }),
        ("Note", VocabularyError::DuplicateName { name: s("Note") }),
    ];
    for (name, expected) in rows {
        let mut v = Vocabulary::builder("v");
        v.node("Note");
        v.native(name, signature.clone(), nothing);
        single(v, expected);
    }
}
