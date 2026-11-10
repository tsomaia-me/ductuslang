use symphonyscript::Diagnostic;
use symphonyscript_evolve::{
    Checker, DEFAULT_BUDGET, Guarded, Program, ProgramSource, Signal, copied_codes,
    named_vocabulary, scaled,
};

const FACTORS: [usize; 3] = [2, 3, 4];
const GATE_PROGRAMS: &str = include_str!("scaling_programs.jsonl");

type Placed = (String, String, usize, usize, usize, usize);
type NameKind = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const NAME_KINDS: &[NameKind] = &[
    (
        "top-level instances, twice-declared among them",
        "main",
        &[(
            "main.sym",
            "Clip intro:\n    Note/48\n    Plays: verse\nClip verse:\n    Note/50\n    Plays: intro\nTrack lead\nTrack lead\nClip lone:\n    Plays: nowhere\n",
        )],
    ),
    (
        "cells",
        "main",
        &[(
            "main.sym",
            "state swell(0.5)\nstate glow = swell * 0.8\nstate swell(0.2)\nClip c:\n    Note/60 | velocity=glow brightness=swell\nlet bad = glow + missing\n",
        )],
    ),
    (
        "lets",
        "main",
        &[(
            "main.sym",
            "let base = 48\nlet top = base + 12\nlet top = 3\nClip c:\n    Note/top\n    Note/(base + nothing)\n",
        )],
    ),
    (
        "functions and a member call",
        "main",
        &[(
            "main.sym",
            "fn db(x): 10 ** (x / 20)\nfn twice(v): v * 2\nfn twice(v): v\nlet g = twice(db(3))\nlet m = g.twice\nClip c:\n    Note/(twice(30))\n",
        )],
    ),
    (
        "patterns and their parameters",
        "main",
        &[(
            "main.sym",
            "pattern Riff(default root, velocity = 0.7):\n    Note/root | velocity=velocity\npattern Riff(default root):\n    Note/root\nClip c:\n    Riff/40 | velocity=0.5\n    Riff low /41\n    Plays: Riff\n",
        )],
    ),
    (
        "record types and their fields",
        "main",
        &[(
            "main.sym",
            "state accent(0.9)\ntype Pt:\n    x: i32\n    accent: bool\nlet p = Pt{x: 1, accent: true}\nlet q = p.accent\nlet r = p.x + accent\nlet s = p.y\n",
        )],
    ),
    (
        "as names, paths and to paths",
        "main",
        &[(
            "main.sym",
            "let hook = 2\nlet velocity = 0.5\nstate section(0)\nClip verse:\n    Plays | gain=to.hook.velocity: [chorus, bridge][section]\nClip chorus:\n    Note as hook /72 | velocity=velocity\nClip bridge:\n    Note as hook /67 | velocity=0.6\nRig rig:\n    Lamp as l1 | level=0.5\n    Lamp as l1\nScene sc:\n    Cue/0\n    Dims: rig.l1\n    Dims: rig.l9\n",
        )],
    ),
    (
        "a name equal to a vocabulary kind",
        "main",
        &[(
            "main.sym",
            "pattern Lamp(default x):\n    Note/x\nlet Clip = 3\nClip c:\n    Lamp/1\n",
        )],
    ),
    (
        "a name equal to a vocabulary attribute",
        "main",
        &[(
            "main.sym",
            "state accent(0.9)\nlet lift = accent * 0.5\nlet velocity = 0.5\npattern Riff(default root, velocity = 0.7):\n    Note/root | velocity=velocity accent\nClip c:\n    Note/60 | velocity=accent accent\n    Note/61 | brightness=lift velocity=velocity\n    Riff/40 | velocity=velocity\n",
        )],
    ),
    (
        "a name equal to a vocabulary enum or variant",
        "main",
        &[(
            "main.sym",
            "let Wave = 1\nlet y = Wave + 1\nlet Sine = 2\nlet z = Sine + 1\nOsc o | wave=Saw\nOsc p | wave=Sine\n",
        )],
    ),
    (
        "a name equal to a vocabulary unit",
        "main",
        &[(
            "main.sym",
            "let s = 2\nlet ms = s + 1\nScene sc | hold=4s:\n    Cue/(2s) | fade=ms\n    Cue/(s * 1ms)\n",
        )],
    ),
    (
        "keywords in member position",
        "main",
        &[(
            "main.sym",
            "let to = 1\ntype K:\n    to: i32\n    in: f32\nlet k = K{to: 1, in: 2.0}\nlet j = k.to + 1\nlet h = k.out\n",
        )],
    ),
    (
        "interpolations",
        "main",
        &[(
            "main.sym",
            "let name = \"x\"\nlet n = 3\nTrack t | label=\"v {n} {name} {gone}\"\n",
        )],
    ),
    (
        "faulty lines",
        "main",
        &[(
            "main.sym",
            "state accent(0.9)\nlet lift = accent * (0.5\nlet broken = = 3\nlet y = broken + lift\nClip c:\n    Note/60 | velocity=accent\n    Note/y\n",
        )],
    ),
    (
        "prelude and built-in names",
        "main",
        &[(
            "main.sym",
            "type Hit:\n    note: i32\nlet h = Hit{note: 1}\nlet v = h.note + h.velocity\nfn min(a): a\nlet m = min(2)\nfn abs2(a): a\n",
        )],
    ),
    (
        "names outside ASCII",
        "main",
        &[(
            "main.sym",
            "let \u{97f3} = 1\nlet b = \u{97f3} + 1\nlet e\u{301} = 2\nlet c = \u{e9} + missing\nClip riff:\n    Note as \u{7b26} /60 | velocity=0.8\n                    brightness=0.7\n    Note as \u{7b26} /61\n",
        )],
    ),
    (
        "two files",
        "main",
        &[
            ("a.sym", "let x = 1\nlet y = x + z\n"),
            ("b.sym", "Clip c:\n    Note/x\n    Note/y\nlet x = 2\n"),
        ],
    ),
    (
        "locals equal to top-level names",
        "main",
        &[(
            "main.sym",
            "let x = 1\nfn f(x): x + 1\nlet y = f(2)\nfn g(a, b = 2): a + b\nlet b = 3\nlet r = g(1, b = b)\nClip c:\n    for x in 0..2:\n        Note/x\n",
        )],
    ),
    (
        "a name equal to a native function",
        "native",
        &[(
            "main.sym",
            "fn checksum(a): a + 1\nlet y = checksum(2)\nlet z = text_length(\"ab\")\n",
        )],
    ),
];

fn program(files: &[(&str, &str)]) -> Program {
    Program {
        id: serde_json::json!("test"),
        sources: files
            .iter()
            .map(|(path, text)| ProgramSource {
                path: (*path).to_string(),
                text: (*text).to_string(),
            })
            .collect(),
    }
}

fn checker(vocabulary: &str) -> Checker {
    let vocabulary = named_vocabulary(vocabulary).unwrap_or_else(|error| panic!("{error}"));
    Checker::with_vocabulary(DEFAULT_BUDGET, vocabulary)
        .unwrap_or_else(|failed| panic!("the vocabulary does not compile: {failed:?}"))
}

fn drawn(checker: &Checker, program: &Program) -> Vec<Diagnostic> {
    match checker.compile(program) {
        Guarded::Finished { value, .. } => value,
        Guarded::Panicked { message } => panic!("the compile panicked: {message}"),
        Guarded::TimedOut => panic!("the compile timed out"),
    }
}

fn lines(text: &str) -> usize {
    text.matches('\n').count() + usize::from(!text.is_empty() && !text.ends_with('\n'))
}

fn position(text: &str, offset: u32) -> (usize, usize) {
    let offset = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(text.len());
    let before = text.get(..offset).unwrap_or_default();
    let line = before.matches('\n').count();
    let column = before
        .rsplit_once('\n')
        .map_or(before, |(_, tail)| tail)
        .chars()
        .count();
    (line, column)
}

fn is_whole(program: &Program, diagnostic: &Diagnostic) -> bool {
    diagnostic.span.range.is_empty()
        && diagnostic.span.range.start == 0
        && program
            .ordered_sources()
            .first()
            .is_some_and(|source| source.path == diagnostic.span.file)
}

fn parted(
    original: &Program,
    copy: &Program,
    diagnostics: &[Diagnostic],
    factor: usize,
) -> (Vec<String>, Vec<Vec<Placed>>) {
    let mut whole = Vec::new();
    let mut parts = vec![Vec::new(); factor];
    for diagnostic in diagnostics {
        let code = diagnostic.code.as_str().to_string();
        if is_whole(copy, diagnostic) {
            whole.push(code);
            continue;
        }
        let text_of = |program: &Program| {
            program
                .sources
                .iter()
                .find(|source| source.path == diagnostic.span.file)
                .map(|source| source.text.clone())
                .unwrap_or_default()
        };
        let height = lines(&text_of(original)).max(1);
        let text = text_of(copy);
        let (line, column) = position(&text, diagnostic.span.range.start);
        let (end_line, end_column) = position(&text, diagnostic.span.range.end);
        let part = (line / height).min(factor);
        let base = part * height;
        let placed = (
            diagnostic.span.file.clone(),
            code,
            line - base,
            column,
            end_line.saturating_sub(base),
            end_column,
        );
        if let Some(found) = parts.get_mut(part) {
            found.push(placed);
        } else {
            parts[factor - 1].push(placed);
        }
    }
    whole.sort();
    for part in &mut parts {
        part.sort();
    }
    (whole, parts)
}

fn copy_faults(checker: &Checker, original: &Program, factor: usize) -> Vec<String> {
    let copy = scaled(original, checker.vocabulary(), factor);
    let (whole, parts) = parted(original, original, &drawn(checker, original), 1);
    let (copy_whole, copy_parts) = parted(original, &copy, &drawn(checker, &copy), factor);
    let mut faults = Vec::new();
    if copy_whole != whole {
        faults.push(format!(
            "x{factor}: the whole program draws {copy_whole:?}, the original {whole:?}"
        ));
    }
    for (index, part) in copy_parts.iter().enumerate() {
        if *part != parts[0] {
            faults.push(format!(
                "x{factor}: part {index} draws {part:?}, the original {:?}",
                parts[0]
            ));
        }
    }
    faults
}

#[test]
fn every_part_of_a_scaled_copy_draws_what_the_original_draws_for_every_kind_of_name() {
    for (label, vocabulary, files) in NAME_KINDS {
        let checker = checker(vocabulary);
        let original = program(files);
        assert!(
            !drawn(&checker, &original).is_empty(),
            "{label}: the row draws nothing"
        );
        for factor in FACTORS {
            let faults = copy_faults(&checker, &original, factor);
            assert!(faults.is_empty(), "{label}: {faults:#?}");
        }
    }
}

#[test]
fn every_part_of_a_scaled_copy_renames_the_programs_top_level_names_and_keeps_the_vocabularys() {
    let checker = checker("main");
    let text = "state accent(0.9)\nlet lift = accent\nlet Sine = 1\nClip c:\n    Note/60 | velocity=lift accent\n";
    let copy = scaled(&program(&[("main.sym", text)]), checker.vocabulary(), 3).sources[0]
        .text
        .clone();
    let lines: Vec<&str> = copy.lines().collect();
    assert_eq!(lines.len(), 15, "{copy}");
    let mut cells = Vec::new();
    let mut lifts = Vec::new();
    for part in lines.chunks(5) {
        let cell = part[0]
            .strip_prefix("state ")
            .and_then(|rest| rest.strip_suffix("(0.9)"));
        let read = part[1]
            .strip_prefix("let ")
            .and_then(|rest| rest.split_once(" = "));
        let (Some(cell), Some((lift, read))) = (cell, read) else {
            panic!("{copy}");
        };
        assert_eq!(read, cell, "{copy}");
        assert_eq!(part[2], "let Sine = 1", "{copy}");
        assert!(part[3].starts_with("Clip "), "{copy}");
        assert_eq!(
            part[4],
            format!("    Note/60 | velocity={lift} accent"),
            "{copy}"
        );
        cells.push(cell);
        lifts.push(lift);
    }
    assert_eq!((cells[0], lifts[0]), ("accent", "lift"));
    for names in [&cells, &lifts] {
        let mut distinct = names.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 3, "{copy}");
        assert!(names.iter().all(|name| name.len() == names[0].len()));
    }
}

#[test]
fn copies_of_the_gates_signalled_programs_draw_the_original_codes_once_per_copy_and_no_scaling() {
    let checker = checker("main");
    for line in GATE_PROGRAMS.lines() {
        let original: Program =
            serde_json::from_str(line).unwrap_or_else(|error| panic!("{error}"));
        let id = original.id.clone();
        let first = drawn(&checker, &original);
        for factor in [2, 4] {
            let mut codes: Vec<String> =
                drawn(&checker, &scaled(&original, checker.vocabulary(), factor))
                    .iter()
                    .map(|diagnostic| diagnostic.code.as_str().to_string())
                    .collect();
            codes.sort();
            assert_eq!(
                codes,
                copied_codes(&original, &first, factor),
                "{id} x{factor}"
            );
        }
        assert_eq!(checker.finding(&original, Signal::Scaling), None, "{id}");
    }
}

#[test]
fn a_program_of_many_one_letter_names_yields_every_copy_at_eight_times() {
    let checker = checker("main");
    let text = "state a(1)\nstate b(2)\nstate i(0)\nlet c = [a, b, a + b]\nlet d = c[i % 3] * 2\nlet e = a + b\nlet f = d - e\nlet g = a * b\nlet h = [a, b, g]\nlet j = h[i % 3] * 2\nlet k = g + d\nlet m = k + n\nlet o = p + q\n";
    let original = program(&[("main.sym", text)]);
    let faults = copy_faults(&checker, &original, 8);
    assert!(faults.is_empty(), "{faults:#?}");
}

#[test]
fn an_unknown_kind_and_an_undeclared_name_stay_as_written_in_every_copy() {
    let checker = checker("main");
    let text = "Phrase bass /40 | next=lead weight=0.8\nPhrase lead /55 | next=bass\nCadence ending /52 | weight=swing\n";
    let original = program(&[("main.sym", text)]);
    let copy = scaled(&original, checker.vocabulary(), 4).sources[0]
        .text
        .clone();
    assert_eq!(copy.matches("Phrase ").count(), 8, "{copy}");
    assert_eq!(copy.matches("Cadence ").count(), 4, "{copy}");
    assert_eq!(copy.matches("weight=swing\n").count(), 4, "{copy}");
    assert_eq!(copy.matches(" bass ").count(), 1, "{copy}");
    for factor in FACTORS {
        let faults = copy_faults(&checker, &original, factor);
        assert!(faults.is_empty(), "{faults:#?}");
    }
}
