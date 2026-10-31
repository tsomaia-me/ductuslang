use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{
    bar_vocabulary, dsp_project, envelope_vocabulary, music_project, native_vocabulary,
    pending_spec_examples, spec_examples, vocabulary,
};

const INSERTED: &[&str] = &[
    "(", ")", "[", "]", ":", ",", "+", "-", "/", "x", "1", "when ", "\n", "{", "}", "^", "..",
    "bounded ", "state ", "u8 ", "to.", "bytes ",
];

const ENVELOPE_PROGRAMS: &[&str] = &[
    "Env e | attack=(1s + 3)\n",
    "Env e | attack=((1s + 3) / 2)\n",
    "Env e | attack=(1s + 3ms)\n",
    "Env e | attack=3i64\n",
    "Env e | attack=i64(3)\n",
    "Env e | attack=(1s + 3i32)\n",
    "Env e | release=3i32\n",
    "Env e | attack=(3i32 * 2.5)\n",
    "let p = 3\nEnv e | attack=(p + 1s) pitch=p\n",
    "let p = 3i32\nlet c = p > 1\nEnv e | attack=p pitch=p\nEnv f | attack=(if c then 1s else 2s)\n",
    "state d(5ms)\nlet e = d * 2\nEnv a | attack=e release=(d + 1)\n",
];

const BAR_PROGRAMS: &[&str] = &[
    "Bar b | bar_len=(1bars + 1)\n",
    "let d = 2bars + 1\nBar b | bar_len=d beat_len=d\n",
    "let k = (2bars + 1) div 1bars\nBar b | bar_len=k\n",
    "let p = 3i64\nBar b | bar_len=p beat_len=p plain=p\n",
    "fn g(): 1bars + 1\nlet x = g()\nBar b | bar_len=(g()) beat_len=x\n",
    "fn r(n): if n < 1 then 2bars + 1 else r(n - 1)\nBar b | bar_len=(r(2)) beat_len=(r(0) + 1)\n",
    "state n(2)\nBar b | bar_len=(n + 1bars) beat_len=n\n",
];

const ADDITION_PROGRAMS: &[&str] = &[
    "state note(60)\nstate recent = (recent.past(1, []) + [note])[^8..]\nClip c:\n    Note/(recent[^1])\n",
    "state gate(false)\nstate edge = gate and not gate.past(2)\nClip c:\n    when edge:\n        Note\n",
    "state level(0.0)\nlet l = level * 2\nstate held = if l > 1 then l else held.past(1, 0.0)\nClip c | gain=held:\n    Note\n",
    "let held = [3, 1, 2]\nfn lowest(i): if i >= len(held) then 127 else min(held[i], lowest(i + 1))\nClip c:\n    Note/(lowest(0))\n",
    "fn count(i): if i > 3 then 0 else count(i + 1) + 1\nClip c:\n    Note/(count(0))\n",
    "state i(1)\nlet xs = [10, 20, 30, 40]\nlet s = xs[i..^1] + xs[..1]\nClip c:\n    Note/(s[0] + len(s))\n",
    "let b = bytes(\"GET\")\nlet u: u8[3] = b\nClip c:\n    Note/(i32(u[0]) + u8(3))\n",
    "Clip a:\n    Note as hook\nClip c:\n    Plays | gain=to.hook.velocity: a\n",
    "state k([1, 2]): i32[..4]\nlet e = []\nlet j = if k[0] > 1 then k else e + [3]\nClip c:\n    Note/(j[^1])\n",
];

const NATIVE_PROGRAMS: &[&str] = &[
    "let a = checksum(\"GET\")\nClip c:\n    Note/a\n",
    "state i(0)\nClip c | sequence=(mirror([1, 2, i])):\n    Note/(text_length(\"abc\"))\n",
    "state held([]): i32[..4]\nClip c | sequence=(mirror(held) + [1]):\n    Note/(checksum([u8(1), u8(2)]))\n",
    "Clip c | seq=d.seq gain=len(d.sequence):\n    Note/(d.rows[1][0] + d.seq[^1] + len(d.rows))\nClip d:\n    Note/(d.sequence[0])\n",
];

pub fn corpora() -> Vec<(Vocabulary, Vec<String>)> {
    let pending = pending_spec_examples();
    let mut texts: Vec<String> = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym" && !pending.contains(&example.name()))
        .map(|example| example.text)
        .collect();
    for project in [music_project(), dsp_project()] {
        texts.extend(project.files.into_iter().map(|file| file.text));
    }
    let owned = |programs: &[&str]| programs.iter().map(|text| text.to_string()).collect();
    texts.extend(ADDITION_PROGRAMS.iter().map(|text| text.to_string()));
    vec![
        (vocabulary(), texts),
        (native_vocabulary(), owned(NATIVE_PROGRAMS)),
        (envelope_vocabulary(), owned(ENVELOPE_PROGRAMS)),
        (bar_vocabulary(), owned(BAR_PROGRAMS)),
    ]
}

pub const MUTATIONS: usize = 1 + INSERTED.len();

pub fn mutant(text: &str, (start, end): (usize, usize), mutation: usize) -> Option<String> {
    let mut changed = text.to_string();
    match mutation.checked_sub(1) {
        None if start < end => changed.replace_range(start..end, ""),
        None => return None,
        Some(inserted) => changed.insert_str(start, INSERTED.get(inserted)?),
    }
    Some(changed)
}

pub fn mutants(text: &str, tokens: &[(usize, usize)]) -> Vec<String> {
    tokens
        .iter()
        .flat_map(|token| (0..MUTATIONS).filter_map(|mutation| mutant(text, *token, mutation)))
        .collect()
}
