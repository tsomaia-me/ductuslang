mod support;

use symphonyscript_fixtures::{dsp_project, music_project, spec_examples};

fn round_trips(text: &str) {
    let parsed = support::parsed(text);
    assert_eq!(parsed.syntax().to_string(), text, "{text:?}");
    for diagnostic in &parsed.diagnostics {
        let range = &diagnostic.span.range;
        assert!(
            range.start <= range.end && range.end as usize <= text.len(),
            "{text:?}"
        );
        assert!(text.is_char_boundary(range.start as usize), "{text:?}");
        assert!(text.is_char_boundary(range.end as usize), "{text:?}");
    }
}

struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

const FRAGMENTS: &[&str] = &[
    "input",
    "let",
    "fn",
    "pattern",
    "default",
    "if",
    "then",
    "else",
    "match",
    "when",
    "for",
    "in",
    "as",
    "is",
    "not",
    "and",
    "or",
    "div",
    "to",
    "true",
    "false",
    "i32",
    "f64",
    "bool",
    "Clip",
    "Note",
    "Plays",
    "Starts",
    "Session",
    "Track",
    "Osc",
    "Wave",
    "Saw",
    "verse",
    "x",
    "C#4",
    "_",
    "#",
    "0",
    "60",
    "0.5",
    "2bars",
    "200ms",
    "0x1F",
    "0b12",
    "1e3",
    ".5",
    "1_",
    "\"",
    "\"a{",
    "}",
    "{",
    "\\",
    "(",
    ")",
    "[",
    "]",
    ",",
    ":",
    "::",
    "=",
    "==",
    "!=",
    "|",
    "/",
    ".",
    "..",
    "+",
    "-",
    "*",
    "**",
    "%",
    "<",
    "<=",
    ">",
    ">=",
    "!",
    "'",
    "?",
    "~",
    "@",
    "$",
    "^",
    ";",
    "&",
    " ",
    " ",
    " ",
    "  ",
    "\t",
    "\n",
    "\n",
    "\n    ",
    "\n        ",
    "\r\n",
    "\r",
    "// c",
    "é",
    "e\u{301}",
    "音",
    "\u{feff}",
];

#[test]
fn spec_examples_and_fixture_projects_round_trip() {
    for example in spec_examples() {
        round_trips(&example.text);
    }
    for project in [music_project(), dsp_project()] {
        for file in project.files {
            round_trips(&file.text);
        }
    }
    round_trips("");
    round_trips("\n\n");
    round_trips("    ");
}

#[test]
fn random_sources_round_trip() {
    let mut random = Random(0x9E37_79B9_7F4A_7C15);
    for _ in 0..3000 {
        let pieces = 1 + random.below(40);
        let mut text = String::new();
        for _ in 0..pieces {
            text.push_str(FRAGMENTS[random.below(FRAGMENTS.len())]);
        }
        round_trips(&text);
    }
}

#[test]
fn mutated_spec_examples_round_trip() {
    let examples: Vec<String> = spec_examples()
        .into_iter()
        .filter(|example| example.tag.starts_with("sym"))
        .map(|example| example.text)
        .collect();
    let mut random = Random(0x2545_F491_4F6C_DD1D);
    for _ in 0..3000 {
        let mut text = examples[random.below(examples.len())].clone();
        for _ in 0..1 + random.below(4) {
            let chars: Vec<(usize, char)> = text.char_indices().collect();
            if chars.is_empty() {
                break;
            }
            let at = chars[random.below(chars.len())].0;
            match random.below(3) {
                0 => {
                    let end = chars
                        .iter()
                        .map(|(offset, c)| offset + c.len_utf8())
                        .find(|end| *end > at)
                        .unwrap_or(text.len());
                    text.replace_range(at..end, "");
                }
                1 => text.insert_str(at, FRAGMENTS[random.below(FRAGMENTS.len())]),
                _ => {
                    let len = 1 + random.below(12);
                    let end = (at + len).min(text.len());
                    let end = (end..=text.len())
                        .find(|end| text.is_char_boundary(*end))
                        .unwrap_or(text.len());
                    let piece = text[at..end].to_string();
                    text.insert_str(text.len(), &piece);
                }
            }
        }
        round_trips(&text);
    }
}
