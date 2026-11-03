use symphonyscript_fixtures::{spec_examples, vocabulary};

use super::binder_kind::BinderKind;
use super::name_kind::NameKind;
use super::op::Op;
use super::{FlatIr, NONE, build_flat_file};
use crate::semantics::ir::Hir;
use crate::syntax::front_end_goldens::source;
use crate::syntax::parse;

fn ir_of(text: &str) -> FlatIr {
    let vocabulary = vocabulary();
    let parsed = parse(&source(text), &vocabulary);
    let mut ir = FlatIr::default();
    assert_eq!(
        build_flat_file(&mut ir, "main.sym", &parsed, &vocabulary),
        Ok(())
    );
    ir
}

fn occurrences(ir: &FlatIr, text: &str) -> Vec<(u32, u32, NameKind)> {
    let file = ir.files.first().copied().expect("one file");
    ir.name_order
        .get(file.names.0 as usize..file.names.1 as usize)
        .unwrap_or_default()
        .iter()
        .filter_map(|at| ir.names.get(*at as usize))
        .filter(|name| ir.symbols.text(name.sym) == text)
        .map(|name| (name.lo, name.hi, name.kind))
        .collect()
}

fn at(program: &str, piece: &str, nth: usize) -> (u32, u32) {
    let start = program
        .match_indices(piece)
        .nth(nth)
        .map(|(start, _)| start)
        .expect("the piece stands in the program");
    let start = u32::try_from(start).unwrap_or(u32::MAX);
    (start, start + u32::try_from(piece.len()).unwrap_or(0))
}

const ANNOTATED: [(&str, Option<BinderKind>); 9] = [
    ("ForExprHead", Some(BinderKind::Accumulator)),
    ("CellDecl", Some(BinderKind::Cell)),
    ("LetDecl", Some(BinderKind::Let)),
    ("LocalLet", Some(BinderKind::LocalLet)),
    ("Param", Some(BinderKind::FnParam)),
    ("PatternParam", Some(BinderKind::PatternParam)),
    ("LambdaParam", None),
    ("GenericParam", None),
    ("LoopHead", Some(BinderKind::PlacementLoop)),
];

#[test]
fn every_annotated_production_of_the_spec_names_a_binder_kind_that_holds_its_annotation_root() {
    let mut found = Vec::new();
    for example in spec_examples()
        .into_iter()
        .filter(|example| example.tag == "ebnf")
    {
        let mut production = String::new();
        for line in example.text.lines() {
            if let Some((name, _)) = line.split_once("::=")
                && !line.starts_with(' ')
            {
                production = name.trim().to_string();
            }
            if line.contains("(':' Type)?") && !found.contains(&production) {
                found.push(production.clone());
            }
        }
    }
    found.sort();
    let mut listed: Vec<String> = ANNOTATED.iter().map(|(name, _)| name.to_string()).collect();
    listed.sort();
    assert_eq!(found, listed);
    let annotated = [
        ("state fine(0): f32\n", BinderKind::Cell, "f32"),
        ("let x: i64 = 1\n", BinderKind::Let, "i64"),
        (
            "Rig r:\n    for i: i32 in 0..3:\n        Lamp/0.5\n",
            BinderKind::PlacementLoop,
            "i32",
        ),
        (
            "Rig r:\n    [for i: i32 in 0..3: Lamp/0.5]\n",
            BinderKind::BundleLoop,
            "i32",
        ),
        (
            "state held([]): i32[..10]\nstate high = for i: i64, x in held into kept: i32[..10] from []: kept\n",
            BinderKind::ForExprLoop,
            "i64",
        ),
        (
            "state held([]): i32[..10]\nstate high = for i: i64, x in held into kept: i32[..10] from []: kept\n",
            BinderKind::Accumulator,
            "i32[..10]",
        ),
    ];
    for (program, kind, written) in annotated {
        let ir = ir_of(program);
        let binder = ir
            .binders
            .iter()
            .find(|binder| binder.kind == kind && binder.annotation != NONE)
            .unwrap_or_else(|| panic!("{kind:?} holds no annotation in {program}"));
        let mut root = binder.annotation;
        let base = loop {
            match ir.node(root).expect("the annotation is a node") {
                (Op::TyArray | Op::TyVar, inner, _, _) => root = inner,
                _ => break ir.span(root).expect("the base has a span"),
            }
        };
        let start = program.rfind(written).expect("the annotation is written");
        let start = u32::try_from(start).unwrap_or(u32::MAX);
        let end = start + u32::try_from(written.len()).unwrap_or(0);
        assert_eq!(base, (start, end), "{kind:?} in {program}");
    }
    let unbuilt = [
        "fn shift(x, by: i32): x + by\n",
        "pattern Lift(gain: f32 = 1.0):\n    Lamp/gain\n",
        "fn f(x):\n    let y: i32 = x\n    y\n",
        "let inc = (x: i32): x + 1\n",
        "fn first[T](xs: T[3]) -> T: xs[0]\n",
    ];
    for program in unbuilt {
        let ir = ir_of(program);
        let mut hir = Hir::default();
        assert_eq!(super::adapt_file(&mut hir, &ir, 0), Ok(()));
        assert!(
            hir.drawn
                .iter()
                .any(|(code, _, _)| *code == crate::diagnostics::Code::UnbuiltConstruct),
            "{program}"
        );
    }
}

#[test]
fn a_name_is_found_at_every_occurrence_in_offset_order() {
    let arp = "pattern Arp(default root, steps = 4, gap = 4) -> (Note*):\n    for i in 0..steps:\n        Note/(root + i * gap)\n\nClip verse:\n    Arp/48\n    Arp/60 | steps=8 gap=3\n";
    assert_eq!(
        occurrences(&ir_of(arp), "Arp"),
        vec![
            (
                at(arp, "Arp", 0).0,
                at(arp, "Arp", 0).1,
                NameKind::Declaration
            ),
            (at(arp, "Arp", 1).0, at(arp, "Arp", 1).1, NameKind::ItemKind),
            (at(arp, "Arp", 2).0, at(arp, "Arp", 2).1, NameKind::ItemKind),
        ]
    );
    let hook = "Clip verse:\n    Note as hook /65\nClip c | gain=verse.hook.velocity:\n    Plays | gain=to.hook.velocity: verse\n";
    assert_eq!(
        occurrences(&ir_of(hook), "hook"),
        vec![
            (
                at(hook, "hook", 0).0,
                at(hook, "hook", 0).1,
                NameKind::AsName
            ),
            (
                at(hook, "hook", 1).0,
                at(hook, "hook", 1).1,
                NameKind::PathSegment
            ),
            (
                at(hook, "hook", 2).0,
                at(hook, "hook", 2).1,
                NameKind::PathSegment
            ),
        ]
    );
    let chord = "type Chord:\n    root: i32\n\nlet chord = Chord {root: 60}\nlet r = chord.root\n";
    assert_eq!(
        occurrences(&ir_of(chord), "root"),
        vec![
            (
                at(chord, "root", 0).0,
                at(chord, "root", 0).1,
                NameKind::Declaration
            ),
            (
                at(chord, "root", 1).0,
                at(chord, "root", 1).1,
                NameKind::FieldKey
            ),
            (
                at(chord, "root", 2).0,
                at(chord, "root", 2).1,
                NameKind::PathSegment
            ),
        ]
    );
    let scale = "let s = scale(degree=3, rot=48)\n";
    assert_eq!(
        occurrences(&ir_of(scale), "rot"),
        vec![(
            at(scale, "rot", 0).0,
            at(scale, "rot", 0).1,
            NameKind::NamedArgument
        )]
    );
    let channel = "Rig r:\n    Lamp | channel=1\n";
    assert_eq!(
        occurrences(&ir_of(channel), "channel"),
        vec![(
            at(channel, "channel", 0).0,
            at(channel, "channel", 0).1,
            NameKind::AttributeName
        )]
    );
}
