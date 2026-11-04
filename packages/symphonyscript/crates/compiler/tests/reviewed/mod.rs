pub type Reviewed = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

pub const REVIEWED: &[Reviewed] = &[
    (
        "464",
        "\nstate a(1)\nstate b(2)\nstate i(0)\n\nlet s = a + b\nlet c = [a, b, s]\n",
        &[("reactive-let", "c"), ("reactive-let", "s")],
    ),
    (
        "464",
        "\nstate a(1)\nlet b = a + 0.5\nlet gate = b > 1\n\nClip c:\n    Note/60 | velocity=gate\n",
        &[
            ("reactive-let", "b"),
            ("reactive-let", "gate"),
            ("type-mismatch", "gate"),
        ],
    ),
    (
        "464",
        "state f(false)\nlet v = if f then [60, 64] else [62, 65]\nlet k = 1 + 1\n\nClip melody:\n    Note/(v[0])\n    Note/(v[k])\n",
        &[("index-out-of-range", "k"), ("reactive-let", "v")],
    ),
    (
        "464",
        "\nstate a(1)\nstate b(2)\nstate i(0)\nlet c = [a, b, a + b]\nstate i(i + 1)\nlet d = c * 2\n",
        &[
            ("array-operand", "c * 2"),
            ("duplicate-name", "i"),
            ("reactive-let", "c"),
            ("reactive-let", "d"),
        ],
    ),
    (
        "464",
        "\nstate a(2)\nstate b(3)\nstate i(0)\nlet c = [b, a, b + a]\nlet mutated = [c, a * 2, b - 1]\n",
        &[
            ("ragged-array", "[c, a * 2, b - 1]"),
            ("reactive-let", "c"),
            ("reactive-let", "mutated"),
        ],
    ),
    (
        "464",
        "\nstate base_pitch(48.0)\nlet pitch_offset = base_pitch * 0.5\n\nfn fifth(p):\n    p + 7\n\nClip melody:\n    Note/(base_pitch + 7)\n    Note/(fifth(base_pitch))\n    Note/(base_pitch + pitch_offset)\n\nSynth lead:\n    Osc | freq=(base_pitch * 2.5)\n",
        &[
            ("reactive-let", "pitch_offset"),
            ("type-mismatch", "base_pitch + 7"),
            ("type-mismatch", "base_pitch + pitch_offset"),
            ("type-mismatch", "p + 7"),
        ],
    ),
    (
        "464",
        "\nstate a(2)\nstate b(3)\nstate i(0)\n\nlet c = [b, a, b + a]\nlet mutated = [b * 2, a + 1, c[i] + 1]\n\nClip main:\n    Note/1 | velocity=a\n    Note/2 | velocity=b\n    Note/3 | velocity=c[i]\n",
        &[("reactive-let", "c"), ("reactive-let", "mutated")],
    ),
    (
        "476",
        "let q = 1\nClip k:\n    Note/((true is q)[if true then [1, 2][5] else [1, 2]])\n",
        &[
            ("not-an-array", "true is q"),
            ("operand-type", "true is q"),
            ("type-mismatch", "if true then [1, 2][5] else [1, 2]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip k:\n    Note/([1, 2][true..])\n",
        &[("type-mismatch", "[1, 2][true..]"), ("index-type", "true")],
    ),
    (
        "476",
        "Clip k:\n    Note/(true[true[true]..])\n",
        &[("not-an-array", "true"), ("not-an-array", "true")],
    ),
    (
        "476",
        "Clip k:\n    Note/(true[[1]..])\n",
        &[("index-type", "true[[1]..]"), ("not-an-array", "true")],
    ),
    (
        "476",
        "Clip k:\n    Note/(if 1 then (if [1, 2][5] then 1 else 2) else 2)\n",
        &[("condition-type", "1"), ("condition-type", "[1, 2][5]")],
    ),
    (
        "476",
        "Clip k:\n    Note/(if 1 then (1 + true) else 2)\n",
        &[("condition-type", "1"), ("operand-type", "1 + true")],
    ),
    (
        "476",
        "let q = 1\nClip k:\n    Note/((if true then q else i32(true))[[1, 2][5] / 300000000000])\n",
        &[
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
            ("not-an-array", "if true then q else i32(true)"),
        ],
    ),
    (
        "476",
        "state i(1)\nlet xs = [10, 20, 30, 40]\nlet s = xs[i..^1] + xs[..1]Clip c:\n    Note/(s[0] + s.size)\n",
        &[("let-syntax", "Clip")],
    ),
    (
        "476",
        "state k([1, 2]): i32\n[..4]\nlet e = []\nlet j = if k[0] > 1 then k else e + [3]\nClip c:\n    Note/(j[^1])\n",
        &[
            ("not-an-array", "k"),
            ("reactive-let", "j"),
            ("top-level-syntax", "["),
            ("type-mismatch", "[1, 2]"),
        ],
    ),
    (
        "476",
        "Clip k:\n    Note/(if 1 then [1] else 2)\n",
        &[
            ("condition-type", "1"),
            ("type-mismatch", "if 1 then [1] else 2"),
        ],
    ),
    (
        "476",
        "let q = 1\nClip k:\n    Note/(if 1 then r else 2)\n",
        &[("condition-type", "1"), ("unknown-name", "r")],
    ),
    (
        "455, 464",
        "\nstate a(2)\nstate b(3)\nstate i(0)\n\nlet c0 = b\nlet c1 = a\nlet c2 = b + a\n\nlet m0 = c0 * 2\nlet m1 = c1 + 1\nlet m2 = c2 + 1\n\nlet pick = i == 0 ? c0 : (i == 1 ? c1 : c2)\nlet mutated2 = pick + 1\n\nlet mutated = m0\n",
        &[
            ("reactive-let", "c0"),
            ("reactive-let", "c1"),
            ("reactive-let", "c2"),
            ("reactive-let", "m0"),
            ("reactive-let", "m1"),
            ("reactive-let", "m2"),
            ("reactive-let", "mutated"),
            ("reserved-token", "=="),
            ("reserved-token", "=="),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if 1 > 0 then [1] else [true])[2 ** 63])\n",
        &[
            ("index-out-of-range", "2 ** 63"),
            ("type-mismatch", "if 1 > 0 then [1] else [true]"),
        ],
    ),
    (
        LIMIT_STOP,
        "fn g(x): for i in 0..x into a from 0: a + i\nfn f(x): g(x) + g(x)\nClip c:\n    Note/(f(2))\n    Note/(f(2))\n    Note/(f(2))\n",
        &[
            ("expansion-limit", "for i in 0..x into a from 0: a + i"),
            ("limit-warning", "for i in 0..x into a from 0: a + i"),
        ],
    ),
];

pub const LIMIT_STOP: &str = "limit-stop (H11)";

pub fn reviewed(text: &str) -> Option<(&'static str, Vec<(String, String)>)> {
    REVIEWED
        .iter()
        .find(|(_, program, _)| *program == text)
        .map(|(ledger, _, expected)| {
            let mut expected: Vec<(String, String)> = expected
                .iter()
                .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
                .collect();
            expected.sort();
            (*ledger, expected)
        })
}

pub type Drawn = (String, usize, usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    Added,
    Removed,
    Restated,
}

pub struct Seen<'a> {
    pub program: &'a str,
    pub old: &'a [Drawn],
    pub new: &'a [Drawn],
}

impl Seen<'_> {
    fn text(&self, at: &Drawn) -> &str {
        self.program.get(at.1..at.2).unwrap_or_default()
    }

    fn before(&self, at: &Drawn) -> &str {
        self.program.get(..at.1).unwrap_or_default()
    }

    fn after(&self, at: &Drawn) -> &str {
        self.program.get(at.2..).unwrap_or_default()
    }

    fn line(&self, start: usize) -> usize {
        self.program
            .get(..start)
            .unwrap_or_default()
            .matches('\n')
            .count()
    }
}

fn holds(set: &[Drawn], code: &str, at: &Drawn) -> bool {
    set.iter()
        .any(|drawn| drawn.0 == code && drawn.1 == at.1 && drawn.2 == at.2)
}

fn top_level_let_name(seen: &Seen<'_>, at: &Drawn) -> bool {
    let before = seen.before(at);
    (before == "let " || before.ends_with("\nlet "))
        && !seen.text(at).is_empty()
        && seen
            .text(at)
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
}

fn reads_a_cell_somewhere(program: &str) -> bool {
    program
        .lines()
        .any(|line| line.starts_with("state ") || line.starts_with("input "))
}

fn line_fault(code: &str) -> bool {
    code.ends_with("-syntax")
        || [
            "unexpected-character",
            "reserved-token",
            "reserved-word",
            "unknown-suffix",
            "prefixed-suffix",
            "float-type-suffix",
            "semicolon",
            "unterminated-string",
            "invalid-identifier",
            "unexpected-indent",
            "dedent-mismatch",
        ]
        .contains(&code)
}

fn reactive_let(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change != Change::Added || !reads_a_cell_somewhere(seen.program) {
        return false;
    }
    if at.0 == "reactive-let" {
        return top_level_let_name(seen, at);
    }
    let line = seen.line(at.1);
    matches!(at.0.as_str(), "type-mismatch" | "does-not-fit")
        && seen.line(at.2.saturating_sub(1).max(at.1)) == line
        && seen.new.iter().any(|drawn| {
            drawn.0 == "reactive-let"
                && seen.line(drawn.1) == line
                && top_level_let_name(seen, drawn)
        })
}

fn read_of_a_reactive_let(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change == Change::Restated || at.0 == "reactive-let" || !reads_a_cell_somewhere(seen.program)
    {
        return false;
    }
    let line = seen.line(at.1);
    let mut texts = vec![seen.program.lines().nth(line).unwrap_or_default()];
    if let Some(function) = function_before(seen.program, at.1)
        .and_then(|text| text.strip_prefix("fn "))
        .and_then(|text| text.split_once('('))
        .map(|(name, _)| format!("{name}("))
    {
        texts.extend(
            seen.program
                .lines()
                .filter(|text| !text.starts_with("fn ") && text.contains(&function)),
        );
    }
    seen.new.iter().any(|drawn| {
        drawn.0 == "reactive-let"
            && top_level_let_name(seen, drawn)
            && seen.line(drawn.1) != line
            && texts.iter().any(|text| reads_word(text, seen.text(drawn)))
    })
}

fn faulty_line(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let line = seen.line(at.1);
    change == Change::Removed
        && at.0 == "reactive-let"
        && top_level_let_name(seen, at)
        && seen
            .old
            .iter()
            .any(|drawn| line_fault(&drawn.0) && seen.line(drawn.1) == line)
}

fn beside_a_fault(seen: &Seen<'_>) -> bool {
    !seen.old.is_empty()
}

fn a_condition(seen: &Seen<'_>, at: &Drawn) -> bool {
    seen.before(at).trim_end_matches('(').ends_with("if ")
        && seen.after(at).trim_start_matches(')').starts_with(" then ")
}

fn condition_beside_arms(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if !beside_a_fault(seen) || !a_condition(seen, at) {
        return false;
    }
    match (change, at.0.as_str()) {
        (Change::Added, "condition-type") => true,
        (Change::Removed, "type-mismatch") => holds(seen.new, "condition-type", at),
        _ => false,
    }
}

fn indexed_base(seen: &Seen<'_>, at: &Drawn) -> bool {
    seen.after(at).trim_start_matches(')').starts_with('[')
}

fn index_of_read(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (at, character) in text.char_indices().rev() {
        match character {
            ']' => depth += 1,
            '[' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

fn slice_read_as_an_index(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let text = seen.text(at);
    let Some(open) = index_of_read(text).filter(|_| text.ends_with(']')) else {
        return false;
    };
    let index = (at.1 + open + 1, at.2 - 1);
    let slice = seen.program.get(index.0..index.1).unwrap_or_default();
    change == Change::Added
        && at.0 == "index-type"
        && slice.ends_with(']')
        && slice.contains("..")
        && seen
            .new
            .iter()
            .any(|drawn| drawn.1 >= index.0 && drawn.2 <= index.1 && drawn != at)
}

fn fit_beside_a_range_fault(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let text = seen.text(at);
    let left = seen.before(at).ends_with("] / ")
        && seen
            .new
            .iter()
            .any(|drawn| drawn.0 == "index-out-of-range" && drawn.2 + 4 == at.1);
    let right = seen.after(at).starts_with(" / [")
        && seen.new.iter().any(|drawn| {
            drawn.0 == "index-out-of-range"
                && drawn.1 > at.2
                && seen.line(drawn.1) == seen.line(at.1)
        });
    change == Change::Added
        && at.0 == "does-not-fit"
        && !text.is_empty()
        && text.chars().all(|character| character.is_ascii_digit())
        && (left || right)
}

fn base_not_an_array(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if !beside_a_fault(seen) || !indexed_base(seen, at) {
        return false;
    }
    match (change, at.0.as_str()) {
        (Change::Added, "not-an-array") => !holds(seen.new, "type-mismatch", at),
        (Change::Removed, "type-mismatch") => holds(seen.new, "not-an-array", at),
        _ => false,
    }
}

fn base_not_an_array_beside_the_twin_pass(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Added
        && beside_a_fault(seen)
        && at.0 == "not-an-array"
        && indexed_base(seen, at)
        && holds(seen.old, "type-mismatch", at)
        && holds(seen.new, "type-mismatch", at)
}

fn word_start(program: &str, at: usize) -> bool {
    program
        .get(..at)
        .and_then(|before| before.chars().next_back())
        .is_none_or(|character| !(character.is_alphanumeric() || character == '_'))
}

fn closing(program: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, character) in program.get(open..)?.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + at);
                }
            }
            _ => {}
        }
    }
    None
}

fn float_conversion_operands(program: &str) -> Vec<(usize, usize)> {
    let mut operands = Vec::new();
    for target in ["f32(", "f64("] {
        for (at, _) in program.match_indices(target) {
            if !word_start(program, at) {
                continue;
            }
            let open = at + target.len() - 1;
            let Some(close) = closing(program, open) else {
                continue;
            };
            let operand = program.get(open + 1..close).unwrap_or_default();
            let fixed = ["u8(", "i32(", "i64("].iter().any(|inner| {
                operand
                    .match_indices(inner)
                    .any(|(start, _)| word_start(operand, start))
            });
            if fixed {
                operands.push((open + 1, close));
            }
        }
    }
    operands
}

fn conversion_stops_the_expected_type(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let inside = |(lo, hi): (usize, usize)| at.1 >= lo && at.2 <= hi;
    let operands = float_conversion_operands(seen.program);
    match (change, at.0.as_str()) {
        (Change::Added, "constant-overflow" | "does-not-fit")
        | (Change::Removed, "type-mismatch") => operands.into_iter().any(inside),
        (Change::Removed, "integer-cell") => {
            let name = seen.text(at);
            !name.is_empty()
                && operands.into_iter().any(|(lo, hi)| {
                    seen.program
                        .get(lo..hi)
                        .unwrap_or_default()
                        .split(|character: char| !(character.is_alphanumeric() || character == '_'))
                        .any(|word| word == name)
                })
        }
        _ => false,
    }
}

fn annotated_array_rules(program: &str) -> Vec<(&str, usize, usize)> {
    let mut rules = Vec::new();
    let mut start = 0;
    for line in program.split_inclusive('\n') {
        let text = line.trim_end_matches('\n');
        if let Some((head, rule)) = text
            .strip_prefix("state ")
            .and_then(|tail| tail.split_once(" = "))
            && let Some((name, ty)) = head.split_once(": ")
            && !ty.is_empty()
            && rule.starts_with('[')
        {
            let lo = start + text.len() - rule.len();
            rules.push((name, lo, start + text.len()));
        }
        start += line.len();
    }
    rules
}

fn reads_word(text: &str, name: &str) -> bool {
    text.split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .any(|word| word == name)
}

fn annotated_array_rule(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    annotated_array_rules(seen.program)
        .into_iter()
        .filter(|(_, lo, hi)| holds(seen.new, "type-mismatch", &(String::new(), *lo, *hi)))
        .any(|(name, lo, hi)| match change {
            Change::Added => at.0 == "type-mismatch" && at.1 == lo && at.2 == hi,
            Change::Restated => false,
            Change::Removed => {
                let line = seen.line(at.1);
                (at.1 >= lo && at.2 <= hi)
                    || (line != seen.line(lo)
                        && reads_word(seen.program.lines().nth(line).unwrap_or_default(), name))
            }
        })
}

fn annotated_accumulator(line: &str) -> bool {
    line.split_once(" into ")
        .and_then(|(_, rest)| rest.split_once(' '))
        .is_some_and(|(name, rest)| name.ends_with(':') && rest.contains(" from "))
}

fn annotated_fold(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change != Change::Added || at.0 != "type-mismatch" {
        return false;
    }
    let line = seen
        .program
        .lines()
        .nth(seen.line(at.1))
        .unwrap_or_default();
    let before = seen.before(at);
    let head = before.rsplit('\n').next().unwrap_or_default();
    let loop_name = seen.after(at).starts_with(": ")
        && (head.ends_with("for ") || head.ends_with(", "))
        && line.contains(" in ");
    let start = head.ends_with(" from ") && annotated_accumulator(line);
    let body = head.ends_with(": ") && head.contains(" from ") && annotated_accumulator(line);
    loop_name || start || body
}

fn annotated_array_lets(program: &str) -> Vec<(&str, usize, usize)> {
    let mut lets = Vec::new();
    let mut start = 0;
    for line in program.split_inclusive('\n') {
        let text = line.trim_end_matches('\n');
        if let Some((head, value)) = text
            .strip_prefix("let ")
            .and_then(|tail| tail.split_once(" = "))
            && let Some((name, ty)) = head.split_once(": ")
            && ty.ends_with(']')
        {
            let lo = start + text.len() - value.len();
            lets.push((name, lo, start + text.len()));
        }
        start += line.len();
    }
    lets
}

fn annotated_array_let(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Added
        && matches!(at.0.as_str(), "does-not-fit" | "type-mismatch")
        && annotated_array_lets(seen.program)
            .into_iter()
            .any(|(name, lo, hi)| {
                (at.1 >= lo && at.2 <= hi)
                    || (at.0 == "type-mismatch"
                        && seen
                            .text(at)
                            .strip_prefix(name)
                            .is_some_and(|rest| rest.starts_with('[')))
            })
}

fn function_before(program: &str, at: usize) -> Option<&str> {
    let mut start = program.get(..at)?.rfind('\n').map_or(0, |line| line + 1);
    while program[start..].starts_with(' ') && start > 0 {
        start = program
            .get(..start - 1)?
            .rfind('\n')
            .map_or(0, |line| line + 1);
    }
    program
        .get(start..at)
        .filter(|text| text.starts_with("fn "))
}

fn fold_fault_at_each_call(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Added
        && holds(seen.old, &at.0, at)
        && function_before(seen.program, at.1).is_some_and(|text| {
            text.rfind("for ")
                .is_some_and(|fold| text[fold..].contains(" into "))
        })
}

fn slice_keeps_its_array_type(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Added
        && at.0 == "type-mismatch"
        && seen.text(at).trim_end_matches('\n').ends_with("..]")
        && seen
            .new
            .iter()
            .any(|drawn| drawn.0 == "index-type" && drawn.1 >= at.1 && drawn.2 <= at.2)
}

fn choice_refused_once(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Removed
        && at.0 == "type-mismatch"
        && seen.text(at).starts_with("if ")
        && holds(seen.new, "type-mismatch", at)
}

fn cell_reads(program: &str, name: &str) -> Vec<(usize, usize)> {
    let mut reads = Vec::new();
    let mut start = 0;
    for line in program.split_inclusive('\n') {
        let declares = ["state ", "input "].iter().any(|head| {
            line.strip_prefix(head)
                .and_then(|rest| rest.strip_prefix(name))
                .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
        });
        if !declares {
            for (at, _) in line.match_indices(name) {
                let lo = start + at;
                let hi = lo + name.len();
                let ends = program
                    .get(hi..)
                    .and_then(|rest| rest.chars().next())
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                if word_start(program, lo) && ends {
                    reads.push((lo, hi));
                }
            }
        }
        start += line.len();
    }
    reads
}

fn integer_cell_read_inside_a_refusal(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    let reads = cell_reads(seen.program, name);
    change == Change::Removed
        && at.0 == "integer-cell"
        && !reads.is_empty()
        && reads.iter().all(|(lo, hi)| {
            seen.new.iter().any(|drawn| {
                matches!(
                    drawn.0.as_str(),
                    "type-mismatch"
                        | "not-an-array"
                        | "operand-type"
                        | "condition-type"
                        | "conversion-type"
                ) && drawn.1 <= *lo
                    && *hi <= drawn.2
                    && (drawn.1, drawn.2) != (*lo, *hi)
            })
        })
}

fn integer_cell_in_an_integer_conversion(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    let reads = cell_reads(seen.program, name);
    let operands: Vec<(usize, usize)> = ["u8(", "i32(", "i64("]
        .iter()
        .flat_map(|target| {
            seen.program
                .match_indices(target)
                .filter(|(start, _)| word_start(seen.program, *start))
                .filter_map(|(start, _)| {
                    let open = start + target.len() - 1;
                    closing(seen.program, open).map(|close| (open + 1, close))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    change == Change::Added
        && at.0 == "integer-cell"
        && !reads.is_empty()
        && reads
            .iter()
            .all(|(lo, hi)| operands.iter().any(|(start, end)| start <= lo && hi <= end))
}

fn integer_cell_through_a_call(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    if change != Change::Added || at.0 != "integer-cell" || name.is_empty() {
        return false;
    }
    seen.program
        .lines()
        .filter_map(|line| line.strip_prefix("fn "))
        .filter_map(|line| {
            line.split_once('(')
                .map(|(function, _)| format!("{function}("))
        })
        .any(|function| {
            seen.program
                .match_indices(&function)
                .filter(|(start, _)| {
                    word_start(seen.program, *start) && !seen.program[..*start].ends_with("fn ")
                })
                .filter_map(|(start, _)| {
                    let open = start + function.len() - 1;
                    closing(seen.program, open).map(|close| (open, close))
                })
                .any(|(open, close)| {
                    reads_word(seen.program.get(open + 1..close).unwrap_or_default(), name)
                })
        })
}

fn declares_an_integer_annotated_cell(program: &str) -> bool {
    program.lines().any(|line| {
        ["state ", "input "]
            .iter()
            .any(|head| line.starts_with(head))
            && [": u8", ": i32", ": i64"]
                .iter()
                .any(|ty| line.trim_end().ends_with(ty))
    })
}

fn computes_in_the_annotations_type(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let line = seen
        .program
        .lines()
        .nth(seen.line(at.1))
        .unwrap_or_default();
    let annotated = line
        .strip_prefix("let ")
        .and_then(|tail| tail.split_once(" = "))
        .and_then(|(head, _)| head.split_once(": "))
        .is_some_and(|(_, ty)| ["u8", "i32", "i64"].contains(&ty));
    let text = seen.text(at);
    matches!(change, Change::Removed | Change::Restated)
        && at.0 == "does-not-fit"
        && annotated
        && !text.is_empty()
        && text.chars().all(|character| character.is_ascii_digit())
        && declares_an_integer_annotated_cell(seen.program)
        && seen
            .new
            .iter()
            .any(|drawn| drawn.0 == "reactive-let" && seen.line(drawn.1) == seen.line(at.1))
}

fn branches_beside_a_condition(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let text = seen.text(at);
    let opened = text.len() - text.trim_start_matches('(').len();
    let Some(rest) = text.get(opened..).and_then(|tail| tail.strip_prefix("if ")) else {
        return false;
    };
    let start = at.1 + opened + 3;
    let condition = start + rest.len() - rest.trim_start_matches('(').len();
    change == Change::Added
        && at.0 == "type-mismatch"
        && seen
            .new
            .iter()
            .any(|drawn| drawn.0 == "condition-type" && (start..=condition).contains(&drawn.1))
}

pub struct Class {
    pub ledger: &'static str,
    pub rule: &'static str,
    explains: fn(&Seen<'_>, Change, &Drawn) -> bool,
}

impl Class {
    pub fn named(&self) -> String {
        format!("{}: {}", self.ledger, self.rule)
    }
}

pub const CLASSES: &[Class] = &[
    Class {
        ledger: "464",
        rule: "a top-level let reading a cell, directly or through another let, draws reactive-let beside any fault in its value or its annotation, and its value's own type-mismatch or a compile-time operand's does-not-fit against its runtime operation's type beside it",
        explains: reactive_let,
    },
    Class {
        ledger: "464",
        rule: "a read of a top-level let reading a cell keeps the let's known type, so the expression reading it, directly or through a call it is passed to, draws the faults that type gives beside the let's reactive-let, where the old engine read the let as a placeholder",
        explains: read_of_a_reactive_let,
    },
    Class {
        ledger: "455",
        rule: "a let declared on a line holding a syntax fault is a placeholder and draws no reactive-let",
        explains: faulty_line,
    },
    Class {
        ledger: "476",
        rule: "condition-type at a choice's non-bool condition beside its mismatched or placeholder arms, in place of the old engine's type-mismatch there",
        explains: condition_beside_arms,
    },
    Class {
        ledger: "476",
        rule: "not-an-array at an indexed or sliced base that is not an array, in place of the old engine's type-mismatch there",
        explains: base_not_an_array,
    },
    Class {
        ledger: "476, the twin pass's (R4b)",
        rule: "not-an-array at a refused bool base beside the type-mismatch the twin pass adds there",
        explains: base_not_an_array_beside_the_twin_pass,
    },
    Class {
        ledger: "476",
        rule: "index-type at a read whose index is a slice keeping its array type beside its refused bound",
        explains: slice_read_as_an_index,
    },
    Class {
        ledger: "476",
        rule: "does-not-fit at a division's literal operand, on either side, beside its sibling's index-out-of-range",
        explains: fit_beside_a_range_fault,
    },
    Class {
        ledger: "476",
        rule: "an annotated derived cell whose rule is an array draws its type-mismatch at the rule, its reads typed by the annotation and the rule checked in its own type, in place of the old engine's faults at its reads and inside its rule",
        explains: annotated_array_rule,
    },
    Class {
        ledger: "476",
        rule: "an annotated fold accumulator's start and body, and an annotated loop name's elements, draw type-mismatch where they do not have the annotated type (SPEC §4.5), which the old engine misses",
        explains: annotated_fold,
    },
    Class {
        ledger: "476",
        rule: "a top-level let with an array annotation fixes its whole value's type: each element that does not have or fit the element type draws at itself whichever element a read selects, and an element read keeps the fixed element type, which the old engine misses",
        explains: annotated_array_let,
    },
    Class {
        ledger: "478",
        rule: "a float conversion's operand holding a fixed-type operand computes in its own type, so its own overflow or fit draws inside it, and the old engine's integer-cell warning and widening type-mismatch from the float target flowing into it do not",
        explains: conversion_stops_the_expected_type,
    },
    Class {
        ledger: "479",
        rule: "a fault inside a fold in a function body draws at each call site with that site's chain, the body checked as inlined at the call (SPEC §6.3), where the old engine draws it at the first site alone",
        explains: fold_fault_at_each_call,
    },
    Class {
        ledger: "411",
        rule: "a choice whose arms the position refuses as one value draws one type-mismatch at the choice, where the old engine draws a second one there for the same fault (ledger 385's rule, ARCHITECTURE §7: one fault draws one diagnostic)",
        explains: choice_refused_once,
    },
    Class {
        ledger: "480",
        rule: "an integer host-fed cell whose every use expects a float draws its integer-cell warning when a use reaches the float position through a call's argument, the body checked as inlined at the call (SPEC §4.4, §6.3), which the old engine misses",
        explains: integer_cell_through_a_call,
    },
    Class {
        ledger: "478",
        rule: "an integer conversion's operand computes in its own type, so an integer host-fed cell read there beside a float operand is a use that expects a float and draws its integer-cell warning (SPEC §4.2, §4.4), where the old engine flows the integer target into it",
        explains: integer_cell_in_an_integer_conversion,
    },
    Class {
        ledger: "476",
        rule: "an integer host-fed cell whose every read lies inside a refused value draws no integer-cell warning, since the program with the refused value repaired reads the cell nowhere, where the old engine draws it from a refused comparison",
        explains: integer_cell_read_inside_a_refusal,
    },
    Class {
        ledger: "476",
        rule: "a slice whose bound is refused keeps its array type, so the position that holds one value draws its type-mismatch at the slice beside the bound's index-type (ARCHITECTURE §7: a value whose type an error leaves known keeps that type), which the old engine misses",
        explains: slice_keeps_its_array_type,
    },
    Class {
        ledger: "476",
        rule: "a reactive let annotated with an integer type that its cell read widens or saturates into computes its operation in the annotation's type, a cell being read in the expected type (SPEC §4.2, §4.4), so a literal draws does-not-fit against the annotation's type alone, where the old engine checks it in the cell's type",
        explains: computes_in_the_annotations_type,
    },
    Class {
        ledger: "476",
        rule: "a choice whose condition draws condition-type draws its branches' type-mismatch beside it, since the program with the condition repaired draws it (G17, G7's converse), which the old engine misses",
        explains: branches_beside_a_condition,
    },
];

fn annotated_let_value(seen: &Seen<'_>, at: &Drawn) -> bool {
    let line = seen.before(at).rsplit('\n').next().unwrap_or_default();
    let Some((head, rest)) = line
        .strip_prefix("let ")
        .and_then(|tail| tail.split_once(": "))
    else {
        return false;
    };
    let named = !head.is_empty()
        && head
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_');
    let scalar = rest
        .strip_suffix(" = ")
        .is_some_and(|ty| ["u8", "i32", "i64", "f32", "f64", "bool"].contains(&ty));
    named && scalar && seen.after(at).starts_with('\n')
}

fn annotation_beside_the_twin_pass(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Removed
        && at.0 == "type-mismatch"
        && !reads_a_cell_somewhere(seen.program)
        && annotated_let_value(seen, at)
        && seen.new.iter().any(|drawn| drawn.0 == "unknown-name")
}

pub const TWIN_PASS_CLASSES: &[Class] = &[Class {
    ledger: "476, the twin pass's (R4b)",
    rule: "a type-mismatch at an annotated let's value that every value of its placeholder draws, which the twin pass misses because its `state q(false)` twin runs on the old engine and draws reactive-let alone there",
    explains: annotation_beside_the_twin_pass,
}];

pub fn missed_by_the_twin_pass(program: &str, twins: &[Drawn], found: &[Drawn]) -> Explained {
    let seen = Seen {
        program,
        old: twins,
        new: found,
    };
    explained(TWIN_PASS_CLASSES, &seen)
}

pub fn changes(old: &[Drawn], new: &[Drawn]) -> Vec<(Change, Drawn)> {
    let mut left: Vec<Drawn> = old.to_vec();
    let mut changes = Vec::new();
    for drawn in new {
        match left.iter().position(|kept| kept == drawn) {
            Some(at) => {
                left.swap_remove(at);
            }
            None => changes.push((Change::Added, drawn.clone())),
        }
    }
    changes.extend(left.into_iter().map(|drawn| (Change::Removed, drawn)));
    changes
}

pub type Explained = Result<Vec<&'static Class>, Vec<(Change, Drawn)>>;

fn explained(classes: &'static [Class], seen: &Seen<'_>) -> Explained {
    let mut used: Vec<&'static Class> = Vec::new();
    let mut unexplained = Vec::new();
    for (change, at) in changes(seen.old, seen.new) {
        match classes
            .iter()
            .find(|class| (class.explains)(seen, change, &at))
        {
            Some(class) => {
                if !used.iter().any(|seen| std::ptr::eq(*seen, class)) {
                    used.push(class);
                }
            }
            None => unexplained.push((change, at)),
        }
    }
    if unexplained.is_empty() {
        Ok(used)
    } else {
        Err(unexplained)
    }
}

pub fn moved(program: &str, old: &[Drawn], new: &[Drawn]) -> Explained {
    explained(CLASSES, &Seen { program, old, new })
}

pub fn restated(program: &str, old: &[Drawn], new: &[Drawn], at: &Drawn) -> bool {
    let seen = Seen { program, old, new };
    CLASSES
        .iter()
        .any(|class| (class.explains)(&seen, Change::Restated, at))
}
