use symphonyscript_core::{Path, Segment};

#[test]
fn path_joins_segments_with_dots() {
    let path = Path {
        segments: vec![
            Segment::Named("verse".into()),
            Segment::Anonymous {
                name: "Arp".into(),
                index: 0,
            },
            Segment::Anonymous {
                name: "Note".into(),
                index: 2,
            },
        ],
    };
    assert_eq!(path.to_string(), "verse.Arp:0.Note:2");
    assert_eq!(Path::default().to_string(), "");
}
