pub type Reviewed = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

pub const REVIEWED: &[Reviewed] = &[
    (
        "464",
        "state base([60, 62])\nlet doubled = base * 2\n\nSession main:\n    Includes: keys\n\nTrack keys:\n    Piano\n    Starts: melody\n\nClip melody:\n    Note/(doubled[1])\n",
        &[("array-operand", "base * 2"), ("reactive-let", "doubled")],
    ),
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
            ("type-mismatch", "if k[0] > 1 then k else e + [3]"),
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
        "476",
        "state sec(0)\nSession s:\n    Clip as a:\n        Plays | gain=to.seq[0]: b\n        Plays | gain=to.seq.size: [b, c][sec]\n        Plays | gain=to.seq[3]: [b, c][sec]\n        Plays | gain=to.seq[0]: c\n    Clip as b\n    Clip as c | seq=[1, 2, 3]\n    Clip as d | seq=(if sec > 0 then b.seq else c.seq)\n    Clip as e | seq=(if sec > 0 then b.seq else c.seq) + [1, 2]\n",
        &[
            ("child-not-accepted", "Clip as a"),
            ("child-not-accepted", "Clip as b"),
            ("child-not-accepted", "Clip as c | seq=[1, 2, 3]"),
            (
                "child-not-accepted",
                "Clip as d | seq=(if sec > 0 then b.seq else c.seq)",
            ),
            (
                "child-not-accepted",
                "Clip as e | seq=(if sec > 0 then b.seq else c.seq) + [1, 2]",
            ),
            ("to-candidates", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
            ("unknown-attribute", "seq"),
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
    (
        LOOP_HEAD_STOP,
        "fn f(x): x\nClip c:\n    for k in 0..(f(0) + f(0) + f(0) + f(0) + f(0) + f(0) + f(0)):\n        Note\n",
        &[("limit-warning", "f(0)")],
    ),
    (
        LOOP_HEAD_STOP,
        "fn f(x): x\nClip c:\n    for k in 0..(f(0) + f(0) + f(0) + f(0) + f(0) + f(0) + f(0) + f(0) + f(0) + f(0) + f(0) + f(0)):\n        Note\n",
        &[("inline-limit", "f(0)"), ("limit-warning", "f(0)")],
    ),
];

pub const LIMIT_STOP: &str = "limit-stop (H11)";

pub const LOOP_HEAD_STOP: &str = "limit-stop (loop head, source order)";

pub type Carried = (
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static [(&'static str, &'static str)],
);

pub const CARRIED_TO_R4B: &[Carried] = &[
    (
        "Clip c:\n    Note | velocity=(if (300000000000 and 300000000000) then (q / 1.5) else (if [1, 2] then Wave::Saw else [1, 2][5]))\n\nstate q(0): i32\n",
        &[
            ("operand-type", "300000000000 and 300000000000"),
            ("condition-type", "if [1, 2] then Wave::Saw else [1, 2][5]"),
            ("type-mismatch", "if [1, 2] then Wave::Saw else [1, 2][5]"),
        ],
        &[
            ("operand-type", "300000000000 and 300000000000"),
            ("condition-type", "if [1, 2] then Wave::Saw else [1, 2][5]"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 and 300000000000) then (q / 1.5) else (if [1, 2] then Wave::Saw else [1, 2][5]))\n\nstate q(0): u8\n",
        &[
            ("operand-type", "300000000000 and 300000000000"),
            ("condition-type", "if [1, 2] then Wave::Saw else [1, 2][5]"),
            ("type-mismatch", "if [1, 2] then Wave::Saw else [1, 2][5]"),
        ],
        &[
            ("operand-type", "300000000000 and 300000000000"),
            ("condition-type", "if [1, 2] then Wave::Saw else [1, 2][5]"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 and 300000000000) then (q / 1.5) else (if [1, 2] then Wave::Saw else [1, 2][5]))\n\nstate q(0): i64\n",
        &[
            ("operand-type", "300000000000 and 300000000000"),
            ("condition-type", "if [1, 2] then Wave::Saw else [1, 2][5]"),
            ("type-mismatch", "if [1, 2] then Wave::Saw else [1, 2][5]"),
        ],
        &[
            ("operand-type", "300000000000 and 300000000000"),
            ("condition-type", "if [1, 2] then Wave::Saw else [1, 2][5]"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=((q[[1, 2][5]..])[if true then q else 1.5])\n\nstate q([]): i32[..4]\n",
        &[
            ("index-out-of-range", "5"),
            ("type-mismatch", "if true then q else 1.5"),
        ],
        &[
            ("index-out-of-range", "5"),
            ("type-mismatch", "if true then q else 1.5"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (true < 1) then (q - i32(true)) else (q[[1, 2][5]]))\n\nstate q([]): i32[..4]\n",
        &[
            ("operand-type", "true < 1"),
            ("array-operand", "q - i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("integer-cell", "q"),
            ("operand-type", "true < 1"),
            ("array-operand", "q - i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/((300000000000 - 1.5) % (if [1, 2] then q else i32(true)))\n",
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "let a: i32 = (300000000000 - 1.5) % (if [1, 2] then q else i32(true))\nClip c:\n    Note/(a)\n",
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=((300000000000 - 1.5) % (if [1, 2] then q else i32(true)))\n",
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (300000000000 - 1.5) % (if [1, 2] then q else i32(true))\n",
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if 1 then 300000000000 else [1, 2][5]) then (if true then q else [1, 2][5]) else (1.5 * i32(true)))\n",
        &[
            ("unknown-name", "q"),
            ("condition-type", "if 1 then 300000000000 else [1, 2][5]"),
            ("condition-type", "1"),
            ("type-mismatch", "1.5 * i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("unknown-name", "q"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if i32(true) then 1 else 1.5) then (1 % [1, 2][5]) else (q * true))\n",
        &[
            ("unknown-name", "q"),
            ("condition-type", "if i32(true) then 1 else 1.5"),
            ("condition-type", "i32(true)"),
            ("type-mismatch", "true"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("unknown-name", "q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "let a: i32 = (if 300000000000 then q else q)[300000000000 - 1.5]\nClip c:\n    Note/(a)\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("condition-type", "300000000000"),
            ("index-type", "300000000000 - 1.5"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("condition-type", "300000000000"),
            ("index-type", "300000000000 - 1.5"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if i32(true) then 1 else 1.5) then (1 % [1, 2][5]) else (q * true))\n",
        &[
            ("unknown-name", "q"),
            ("condition-type", "if i32(true) then 1 else 1.5"),
            ("condition-type", "i32(true)"),
            ("type-mismatch", "true"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("unknown-name", "q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (1.5 or i32(true)) then (if [1, 2] then i32(true) else 1.5) else ([1, 2][5] or q))\n",
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then i32(true) else 1.5"),
            ("type-mismatch", "[1, 2][5] or q"),
            ("operand-type", "1.5 or i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("unknown-name", "q"),
            ("condition-type", "if [1, 2] then i32(true) else 1.5"),
            ("type-mismatch", "[1, 2][5] or q"),
            ("operand-type", "1.5 or i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (if 300000000000 then q else q)[300000000000 - 1.5]\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("condition-type", "300000000000"),
            ("index-type", "300000000000 - 1.5"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("condition-type", "300000000000"),
            ("index-type", "300000000000 - 1.5"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0): i32\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0): u8\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0): i64\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0.5): f32\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0.5): f64\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (u8(i32(true))) then (if 300000000000 then 1.5 else q) else (i32(true) / 300000000000))\n\nlet q = false\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "u8(i32(true))"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 300000000000 then 1.5 else q"),
        ],
        &[
            ("conversion-type", "i32(true)"),
            ("condition-type", "300000000000"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if ([1, 2][5] div [1, 2][5]) then (q * 1) else (if 1.5 then i32(true) else true))\n\nlet q = i64(1)\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5] div [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if 1.5 then i32(true) else true"),
        ],
        &[
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("condition-type", "1.5"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if ([1, 2][5] div [1, 2][5]) then (q * 1) else (if 1.5 then i32(true) else true))\n\nlet q = f32(0.5)\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5] div [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if 1.5 then i32(true) else true"),
        ],
        &[
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("condition-type", "1.5"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if ([1, 2][5] div [1, 2][5]) then (q * 1) else (if 1.5 then i32(true) else true))\n\nlet q = 0.5\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5] div [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if 1.5 then i32(true) else true"),
        ],
        &[
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("condition-type", "1.5"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(i32(if 1.5 then q else q))\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(i32(if 1.5 then q else q))\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(i32(if 1.5 then q else q))\n\nlet q = [1]\n",
        &[
            ("array-operand", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 / 300000000000) then (if true then 1 else 1.5) else (300000000000 div 1.5))\n",
        &[
            ("condition-type", "300000000000 / 300000000000"),
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "300000000000"),
            ("operand-type", "300000000000 div 1.5"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "300000000000"),
            ("operand-type", "300000000000 div 1.5"),
        ],
    ),
    (
        "Clip c:\n    Note/((300000000000 - 1.5) % (if [1, 2] then q else i32(true)))\n\nlet q = i64(1)\n",
        &[
            (
                "type-mismatch",
                "(300000000000 - 1.5) % (if [1, 2] then q else i32(true))",
            ),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/((300000000000 - 1.5) % (if [1, 2] then q else i32(true)))\n\nstate q(0.5): f64\n",
        &[
            (
                "type-mismatch",
                "(300000000000 - 1.5) % (if [1, 2] then q else i32(true))",
            ),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5))\n\nstate q(false)\n",
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            (
                "type-mismatch",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("index-out-of-range", "5"),
        ],
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("condition-type", "[1, 2][[1, 2][5]..]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note/(if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5))\n\nlet q = false\n",
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            (
                "type-mismatch",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("index-out-of-range", "5"),
        ],
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("condition-type", "[1, 2][[1, 2][5]..]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0): i32\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0): u8\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0): i64\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0.5): f32\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0.5): f64\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = 1\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = u8(1)\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = i64(1)\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = f32(0.5)\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = 0.5\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = false\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = [1]\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if true then 1.5 else 300000000000) then (i32(true) < 1.5) else (q * q))\n\nlet q = false\n",
        &[
            ("condition-type", "if true then 1.5 else 300000000000"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "q * q"),
        ],
        &[("conversion-type", "i32(true)"), ("operand-type", "q * q")],
    ),
    (
        "let a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\nClip c:\n    Note/(a)\n\nstate q(0): i32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\nClip c:\n    Note/(a)\n\nstate q(0): u8\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\nClip c:\n    Note/(a)\n\nstate q(0): i64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\nClip c:\n    Note/(a)\n\nstate q(0.5): f32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\nClip c:\n    Note/(a)\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "let a: i32 = i32(if 1.5 then q else q)\nClip c:\n    Note/(a)\n\nstate q(false)\n",
        &[
            ("reactive-let", "a"),
            ("conversion-type", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "let a: i32 = i32(if 1.5 then q else q)\nClip c:\n    Note/(a)\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "let a: i32 = i32(if 1.5 then q else q)\nClip c:\n    Note/(a)\n\nlet q = [1]\n",
        &[
            ("array-operand", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "let a: i32 = (300000000000 - 1.5) % (if [1, 2] then q else i32(true))\nClip c:\n    Note/(a)\n\nlet q = i64(1)\n",
        &[
            (
                "type-mismatch",
                "(300000000000 - 1.5) % (if [1, 2] then q else i32(true))",
            ),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "let a: i32 = (300000000000 - 1.5) % (if [1, 2] then q else i32(true))\nClip c:\n    Note/(a)\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "(300000000000 - 1.5) % (if [1, 2] then q else i32(true))",
            ),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "let a: i32 = if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)\nClip c:\n    Note/(a)\n\nstate q(false)\n",
        &[
            ("reactive-let", "a"),
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            (
                "type-mismatch",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("index-out-of-range", "5"),
        ],
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("condition-type", "[1, 2][[1, 2][5]..]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "let a: i32 = if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)\nClip c:\n    Note/(a)\n\nlet q = false\n",
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            (
                "type-mismatch",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("index-out-of-range", "5"),
        ],
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("condition-type", "[1, 2][[1, 2][5]..]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "let a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\nClip c:\n    Note/(a)\n\nstate q(0): i32\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "let a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\nClip c:\n    Note/(a)\n\nstate q(0): u8\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "let a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\nClip c:\n    Note/(a)\n\nstate q(0): i64\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "let a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\nClip c:\n    Note/(a)\n\nstate q(0.5): f32\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "let a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\nClip c:\n    Note/(a)\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if ([1, 2] and [1, 2][5]) then (q * [1, 2][5]) else (if true then q else [1, 2][5]))\n\nstate q(0): i32\n",
        &[
            ("array-operand", "[1, 2] and [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
        ],
        &[
            ("array-operand", "[1, 2] and [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if ([1, 2] and [1, 2][5]) then (q * [1, 2][5]) else (if true then q else [1, 2][5]))\n\nstate q(0): u8\n",
        &[
            ("array-operand", "[1, 2] and [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
        ],
        &[
            ("array-operand", "[1, 2] and [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if ([1, 2] and [1, 2][5]) then (q * [1, 2][5]) else (if true then q else [1, 2][5]))\n\nstate q(0): i64\n",
        &[
            ("array-operand", "[1, 2] and [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
        ],
        &[
            ("array-operand", "[1, 2] and [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2] then [1, 2] else q) then (1.5 % q) else (true[1.5]))\n\nstate q(0): i32\n",
        &[
            ("condition-type", "if [1, 2] then [1, 2] else q"),
            ("type-mismatch", "if [1, 2] then [1, 2] else q"),
            ("not-an-array", "true"),
        ],
        &[
            ("condition-type", "if [1, 2] then [1, 2] else q"),
            ("not-an-array", "true"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2] then [1, 2] else q) then (1.5 % q) else (true[1.5]))\n\nstate q(0): u8\n",
        &[
            ("condition-type", "if [1, 2] then [1, 2] else q"),
            ("type-mismatch", "if [1, 2] then [1, 2] else q"),
            ("not-an-array", "true"),
        ],
        &[
            ("condition-type", "if [1, 2] then [1, 2] else q"),
            ("not-an-array", "true"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2] then [1, 2] else q) then (1.5 % q) else (true[1.5]))\n\nstate q(0): i64\n",
        &[
            ("condition-type", "if [1, 2] then [1, 2] else q"),
            ("type-mismatch", "if [1, 2] then [1, 2] else q"),
            ("not-an-array", "true"),
        ],
        &[
            ("condition-type", "if [1, 2] then [1, 2] else q"),
            ("not-an-array", "true"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0): i32\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0): u8\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0): i64\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0.5): f32\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2]))\n\nstate q(0.5): f64\n",
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("type-mismatch", "[1, 2][5] or true"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\n\nstate q(0): i32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\n\nstate q(0): u8\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\n\nstate q(0): i64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\n\nstate q(0.5): f32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2] then q else 1) then ([1, 2][5] or true) else (if [1, 2] then 1 else [1, 2])\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
            ("type-mismatch", "if [1, 2] then 1 else [1, 2]"),
        ],
        &[
            ("condition-type", "if [1, 2] then q else 1"),
            ("type-mismatch", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1 else [1, 2]"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if ([1, 2][5] div [1, 2][5]) then (q * 1) else (if 1.5 then i32(true) else true))\n\nlet q = i64(1)\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5] div [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if 1.5 then i32(true) else true"),
        ],
        &[
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("condition-type", "1.5"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(if 1.5 then q else q))\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(if 1.5 then q else q))\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(if 1.5 then q else q))\n\nlet q = [1]\n",
        &[
            ("array-operand", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(if 1.5 then q else q)\n\nstate q(false)\n",
        &[
            ("reactive-let", "a"),
            ("conversion-type", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(if 1.5 then q else q)\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(if 1.5 then q else q)\n\nlet q = [1]\n",
        &[
            ("array-operand", "i32(if 1.5 then q else q)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note | velocity=((300000000000 - 1.5) % (if [1, 2] then q else i32(true)))\n\nlet q = i64(1)\n",
        &[
            (
                "type-mismatch",
                "(300000000000 - 1.5) % (if [1, 2] then q else i32(true))",
            ),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (300000000000 - 1.5) % (if [1, 2] then q else i32(true))\n\nlet q = i64(1)\n",
        &[
            (
                "type-mismatch",
                "(300000000000 - 1.5) % (if [1, 2] then q else i32(true))",
            ),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (300000000000 - 1.5) % (if [1, 2] then q else i32(true))\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "(300000000000 - 1.5) % (if [1, 2] then q else i32(true))",
            ),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("condition-type", "if [1, 2] then q else i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5))\n\nstate q(false)\n",
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            (
                "type-mismatch",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("index-out-of-range", "5"),
        ],
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("condition-type", "[1, 2][[1, 2][5]..]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5))\n\nlet q = false\n",
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            (
                "type-mismatch",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("index-out-of-range", "5"),
        ],
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("condition-type", "[1, 2][[1, 2][5]..]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)\n\nstate q(false)\n",
        &[
            ("reactive-let", "a"),
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            (
                "type-mismatch",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("index-out-of-range", "5"),
        ],
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("condition-type", "[1, 2][[1, 2][5]..]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)\n\nlet q = false\n",
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            (
                "type-mismatch",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("index-out-of-range", "5"),
        ],
        &[
            (
                "condition-type",
                "if ([1, 2][[1, 2][5]..]) then (if q then 1 else [1, 2][5]) else (300000000000 < 1.5)",
            ),
            ("condition-type", "[1, 2][[1, 2][5]..]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (1 and q) then (q - 1) else (if [1, 2][5] then 300000000000 else q))\n\nstate q(0): i32\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
            ("operand-type", "1 and q"),
        ],
        &[
            ("operand-type", "1 and q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (1 and q) then (q - 1) else (if [1, 2][5] then 300000000000 else q))\n\nstate q(0): u8\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
            ("operand-type", "1 and q"),
        ],
        &[
            ("operand-type", "1 and q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (1 and q) then (q - 1) else (if [1, 2][5] then 300000000000 else q))\n\nstate q(0): i64\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
            ("integer-cell", "q"),
            ("operand-type", "1 and q"),
        ],
        &[
            ("operand-type", "1 and q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0): i32\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0): u8\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0): i64\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0.5): f32\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true)))\n\nstate q(0.5): f64\n",
        &[
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\n\nstate q(0): i32\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\n\nstate q(0): u8\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\n\nstate q(0): i64\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\n\nstate q(0.5): f32\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "if (if i32(true) then [1, 2][5] else q) then ([1, 2][5] and i32(true)) else (i32(true) % i32(true))",
            ),
            ("condition-type", "if i32(true) then [1, 2][5] else q"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "[1, 2][5] and i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = 1\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = u8(1)\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = i64(1)\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = f32(0.5)\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = 0.5\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = false\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < i32(true)) then (q is [1, 2]) else (if 1 then [1, 2][5] else true))\n\nlet q = [1]\n",
        &[
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if 1 then [1, 2][5] else true"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "q is [1, 2]"),
            ("condition-type", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if true then 1.5 else 300000000000) then (i32(true) < 1.5) else (q * q))\n\nlet q = false\n",
        &[
            ("condition-type", "if true then 1.5 else 300000000000"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "q * q"),
        ],
        &[("conversion-type", "i32(true)"), ("operand-type", "q * q")],
    ),
    (
        "Clip c:\n    Note | velocity=(if (true < 1) then (q - i32(true)) else (q[[1, 2][5]]))\n\nstate q(0): i32\n",
        &[
            ("operand-type", "true < 1"),
            ("conversion-type", "i32(true)"),
            ("not-an-array", "q"),
        ],
        &[
            ("operand-type", "true < 1"),
            ("conversion-type", "i32(true)"),
            ("not-an-array", "q"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (true < 1) then (q - i32(true)) else (q[[1, 2][5]]))\n\nstate q(0): u8\n",
        &[
            ("operand-type", "true < 1"),
            ("conversion-type", "i32(true)"),
            ("not-an-array", "q"),
        ],
        &[
            ("operand-type", "true < 1"),
            ("conversion-type", "i32(true)"),
            ("not-an-array", "q"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (true < 1) then (q - i32(true)) else (q[[1, 2][5]]))\n\nstate q(0): i64\n",
        &[
            ("operand-type", "true < 1"),
            ("conversion-type", "i32(true)"),
            ("not-an-array", "q"),
        ],
        &[
            ("operand-type", "true < 1"),
            ("conversion-type", "i32(true)"),
            ("not-an-array", "q"),
            ("integer-cell", "q"),
        ],
    ),
    (
        "Clip c:\n    Note/(if true then 300000000000 else q)\n\nstate q(0): i32\n",
        &[("does-not-fit", "if true then 300000000000 else q")],
        &[],
    ),
    (
        "Clip c:\n    Note/(if true then 300000000000 else q)\n\nstate q(0): u8\n",
        &[("does-not-fit", "if true then 300000000000 else q")],
        &[],
    ),
    (
        "Clip c:\n    Note/(if true then 300000000000 else q)\n\nstate q(0): i64\n",
        &[("does-not-fit", "if true then 300000000000 else q")],
        &[],
    ),
    (
        "Clip c:\n    Note/(([1, 2] or [1, 2][5])[[1, 2][[1, 2]..]])\n",
        &[
            ("index-type", "([1, 2] or [1, 2][5])[[1, 2][[1, 2]..]]"),
            ("array-operand", "[1, 2] or [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("index-type", "[1, 2][[1, 2]..]"),
            ("not-an-array", "[1, 2] or [1, 2][5]"),
        ],
        &[
            ("array-operand", "[1, 2] or [1, 2][5]"),
            ("type-mismatch", "[1, 2] or [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("index-type", "[1, 2][[1, 2]..]"),
        ],
    ),
    (
        "Clip c:\n    Note/((q div 300000000000)[if i32(true) then true else [1, 2]..])\n\nstate q(0): u8\n",
        &[
            ("not-an-array", "q div 300000000000"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/((q div 300000000000)[if i32(true) then true else [1, 2]..])\n\nlet q = u8(1)\n",
        &[
            ("not-an-array", "q div 300000000000"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/((q div 300000000000)[if i32(true) then true else [1, 2]..])\n\nstate q(0): i64\n",
        &[
            ("not-an-array", "q div 300000000000"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if true then [1, 2][5] else 300000000000) then (if q then 300000000000 else 1.5) else (if 300000000000 then 1 else q))\n\nlet q = false\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "if true then [1, 2][5] else 300000000000"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if 300000000000 then 1 else q"),
        ],
        &[
            ("index-out-of-range", "5"),
            ("condition-type", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if 1 then 300000000000 else [1, 2][5]) then (if true then q else [1, 2][5]) else (1.5 * i32(true)))\n\nlet q = false\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then 300000000000 else [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if true then q else [1, 2][5]"),
        ],
        &[
            ("condition-type", "1"),
            ("type-mismatch", "if true then q else [1, 2][5]"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if 1 then 300000000000 else [1, 2][5]) then (if true then q else [1, 2][5]) else (1.5 * i32(true)))\n\nlet q = [1]\n",
        &[
            ("condition-type", "if 1 then 300000000000 else [1, 2][5]"),
            ("condition-type", "1"),
            ("type-mismatch", "if true then q else [1, 2][5]"),
            ("type-mismatch", "1.5 * i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("condition-type", "1"),
            ("type-mismatch", "if true then q else [1, 2][5]"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0): i32\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0): u8\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0): i64\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0.5): f32\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0.5): f64\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (1 + 1.5) then (true < true) else (if i32(true) then q else [1, 2]))\n\nlet q = [1]\n",
        &[
            ("condition-type", "1 + 1.5"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "true < true"),
        ],
        &[
            ("operand-type", "true < true"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/((if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..]))\n",
        &[
            (
                "array-operand",
                "(if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])",
            ),
            (
                "type-mismatch",
                "(if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])",
            ),
            ("index-type", "[1, 2][[1, 2]..]"),
        ],
        &[("index-type", "[1, 2][[1, 2]..]")],
    ),
    (
        "Clip c:\n    Note/((1.5 / 1) or (if [1, 2] then true else true))\n",
        &[
            (
                "type-mismatch",
                "(1.5 / 1) or (if [1, 2] then true else true)",
            ),
            ("type-mismatch", "1.5 / 1"),
            ("condition-type", "if [1, 2] then true else true"),
        ],
        &[("condition-type", "if [1, 2] then true else true")],
    ),
    (
        "Clip c:\n    Note/((if i32(true) then true else [1, 2][5])[300000000000 / q..])\n\nstate q(0): i32\n",
        &[
            ("type-mismatch", "if i32(true) then true else [1, 2][5]"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-type", "300000000000 / q"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-type", "300000000000 / q"),
        ],
    ),
    (
        "Clip c:\n    Note/((if i32(true) then true else [1, 2][5])[300000000000 / q..])\n\nstate q(0): u8\n",
        &[
            ("type-mismatch", "if i32(true) then true else [1, 2][5]"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-type", "300000000000 / q"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-type", "300000000000 / q"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < 300000000000) then (if [1, 2] then [1, 2] else 1.5) else (if q then [1, 2][5] else 300000000000))\n\nstate q(false)\n",
        &[
            ("condition-type", "if [1, 2] then [1, 2] else 1.5"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if [1, 2] then [1, 2] else 1.5"),
        ],
        &[
            ("condition-type", "if [1, 2] then [1, 2] else 1.5"),
            ("index-out-of-range", "5"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (300000000000 < 300000000000) then (if [1, 2] then [1, 2] else 1.5) else (if q then [1, 2][5] else 300000000000))\n\nlet q = false\n",
        &[
            ("condition-type", "if [1, 2] then [1, 2] else 1.5"),
            ("does-not-fit", "if q then [1, 2][5] else 300000000000"),
            ("type-mismatch", "if [1, 2] then [1, 2] else 1.5"),
        ],
        &[
            ("condition-type", "if [1, 2] then [1, 2] else 1.5"),
            ("does-not-fit", "if q then [1, 2][5] else 300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note/(i32(if 300000000000 then q else q))\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note/(i32(if 300000000000 then q else q))\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note/(i32(if 300000000000 then q else q))\n\nlet q = [1]\n",
        &[
            ("array-operand", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note/(i32(if 1.5 then q else true))\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else true)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(i32(if 1.5 then q else true))\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else true)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/((i32(true) div i32(true)) % (300000000000 * [1, 2][5]))\n",
        &[
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note/((300000000000 % 1) div (if i32(true) then q else 300000000000))\n\nlet q = u8(1)\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/((if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2]))\n",
        &[
            (
                "array-operand",
                "(if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])",
            ),
            (
                "type-mismatch",
                "(if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])",
            ),
            ("type-mismatch", "if [1, 2][5] then true else i32(true)"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "1.5"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "1.5"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = 1\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = u8(1)\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = i64(1)\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = f32(0.5)\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = 0.5\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0): i32\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0): u8\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0): i64\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0.5): f32\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0.5): f64\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(i32(q[1.5..]))\n\nlet q = [1]\n",
        &[("array-operand", "i32(q[1.5..])"), ("index-type", "1.5")],
        &[("index-type", "1.5")],
    ),
    (
        "let a: i32 = if true then 300000000000 else q\nClip c:\n    Note/(a)\n\nstate q(0): i32\n",
        &[(
            "does-not-fit",
            "let a: i32 = if true then 300000000000 else q\n",
        )],
        &[],
    ),
    (
        "let a: i32 = if true then 300000000000 else q\nClip c:\n    Note/(a)\n\nstate q(0): u8\n",
        &[(
            "does-not-fit",
            "let a: i32 = if true then 300000000000 else q\n",
        )],
        &[],
    ),
    (
        "let a: i32 = if true then 300000000000 else q\nClip c:\n    Note/(a)\n\nstate q(0): i64\n",
        &[(
            "does-not-fit",
            "let a: i32 = if true then 300000000000 else q\n",
        )],
        &[],
    ),
    (
        "let a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\nClip c:\n    Note/(a)\n\nstate q(0): i32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "let a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\nClip c:\n    Note/(a)\n\nstate q(0): u8\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "let a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\nClip c:\n    Note/(a)\n\nstate q(0): i64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "let a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\nClip c:\n    Note/(a)\n\nstate q(0.5): f32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "let a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\nClip c:\n    Note/(a)\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "let a: i32 = (if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])\nClip c:\n    Note/(a)\n",
        &[
            (
                "array-operand",
                "(if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])",
            ),
            (
                "type-mismatch",
                "(if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])",
            ),
            ("index-type", "[1, 2][[1, 2]..]"),
        ],
        &[("index-type", "[1, 2][[1, 2]..]")],
    ),
    (
        "let a: i32 = (1.5 / 1) or (if [1, 2] then true else true)\nClip c:\n    Note/(a)\n",
        &[
            (
                "type-mismatch",
                "(1.5 / 1) or (if [1, 2] then true else true)",
            ),
            ("type-mismatch", "1.5 / 1"),
            ("condition-type", "if [1, 2] then true else true"),
        ],
        &[("condition-type", "if [1, 2] then true else true")],
    ),
    (
        "let a: i32 = i32(if 300000000000 then q else q)\nClip c:\n    Note/(a)\n\nstate q(false)\n",
        &[
            ("reactive-let", "a"),
            ("conversion-type", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "let a: i32 = i32(if 300000000000 then q else q)\nClip c:\n    Note/(a)\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "let a: i32 = i32(if 300000000000 then q else q)\nClip c:\n    Note/(a)\n\nlet q = [1]\n",
        &[
            ("array-operand", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "let a: i32 = i32(if 1.5 then q else true)\nClip c:\n    Note/(a)\n\nstate q(false)\n",
        &[
            ("reactive-let", "a"),
            ("conversion-type", "i32(if 1.5 then q else true)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "let a: i32 = i32(if 1.5 then q else true)\nClip c:\n    Note/(a)\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else true)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "let a: i32 = (i32(true) div i32(true)) % (300000000000 * [1, 2][5])\nClip c:\n    Note/(a)\n",
        &[
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "let a: i32 = (300000000000 % 1) div (if i32(true) then q else 300000000000)\nClip c:\n    Note/(a)\n\nlet q = u8(1)\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "let a: i32 = (if q then 300000000000 else i32(true))[q + q..]\nClip c:\n    Note/(a)\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("not-an-array", "if q then 300000000000 else i32(true)"),
            ("operand-type", "q + q"),
            ("reactive-let", "a"),
        ],
        &[
            ("not-an-array", "if q then 300000000000 else i32(true)"),
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "q + q"),
        ],
    ),
    (
        "let a: i32 = (if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])\nClip c:\n    Note/(a)\n",
        &[
            (
                "array-operand",
                "(if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])",
            ),
            (
                "type-mismatch",
                "(if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])",
            ),
            ("type-mismatch", "if [1, 2][5] then true else i32(true)"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "1.5"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "1.5"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\nClip c:\n    Note/(a)\n\nstate q(0): i32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\nClip c:\n    Note/(a)\n\nstate q(0): u8\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\nClip c:\n    Note/(a)\n\nstate q(0): i64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\nClip c:\n    Note/(a)\n\nstate q(0.5): f32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "let a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\nClip c:\n    Note/(a)\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "let a: i32 = i32(q[1.5..])\nClip c:\n    Note/(a)\n\nlet q = [1]\n",
        &[("array-operand", "i32(q[1.5..])"), ("index-type", "1.5")],
        &[("index-type", "1.5")],
    ),
    (
        "let a: i32 = not (if q then i32(true) else 300000000000)\nClip c:\n    Note/(a)\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            (
                "operand-type",
                "not (if q then i32(true) else 300000000000)",
            ),
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "not (if q then i32(true) else 300000000000)",
            ),
        ],
        &[
            ("reactive-let", "a"),
            (
                "operand-type",
                "not (if q then i32(true) else 300000000000)",
            ),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(q div 300000000000)\n\nlet q = u8(1)\n",
        &[("does-not-fit", "300000000000")],
        &[],
    ),
    (
        "Clip c:\n    Note | velocity=(300000000000 div q)\n\nlet q = u8(1)\n",
        &[("does-not-fit", "300000000000")],
        &[],
    ),
    (
        "Clip c:\n    Note | velocity=(300000000000 div i32(true))\n",
        &[
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
        ],
        &[("conversion-type", "i32(true)")],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(true) div 300000000000)\n",
        &[
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
        &[("conversion-type", "i32(true)")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if true then 300000000000 else q\n\nstate q(0): i32\n",
        &[(
            "does-not-fit",
            "let a: i32 = if true then 300000000000 else q\n",
        )],
        &[],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if true then 300000000000 else q\n\nstate q(0): u8\n",
        &[(
            "does-not-fit",
            "let a: i32 = if true then 300000000000 else q\n",
        )],
        &[],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if true then 300000000000 else q\n\nstate q(0): i64\n",
        &[(
            "does-not-fit",
            "let a: i32 = if true then 300000000000 else q\n",
        )],
        &[],
    ),
    (
        "Clip c:\n    Note | velocity=(([1, 2] or [1, 2][5])[[1, 2][[1, 2]..]])\n",
        &[
            ("index-type", "([1, 2] or [1, 2][5])[[1, 2][[1, 2]..]]"),
            ("array-operand", "[1, 2] or [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("index-type", "[1, 2][[1, 2]..]"),
            ("not-an-array", "[1, 2] or [1, 2][5]"),
        ],
        &[
            ("array-operand", "[1, 2] or [1, 2][5]"),
            ("type-mismatch", "[1, 2] or [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("index-type", "[1, 2][[1, 2]..]"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=((q div 300000000000)[if i32(true) then true else [1, 2]..])\n\nlet q = u8(1)\n",
        &[
            ("not-an-array", "q div 300000000000"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
        &[
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0): i32\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0): u8\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0): i64\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0.5): f32\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5]))\n\nstate q(0.5): f64\n",
        &[
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\n\nstate q(0): i32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\n\nstate q(0): u8\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\n\nstate q(0): i64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\n\nstate q(0.5): f32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if q then i32(true) else [1, 2][5]) then (1[q]) else ([1, 2][5] * [1, 2][5])\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if q then i32(true) else [1, 2][5]"),
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("condition-type", "q"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("not-an-array", "1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (1 + 1.5) then (true < true) else (if i32(true) then q else [1, 2]))\n\nlet q = [1]\n",
        &[
            ("condition-type", "1 + 1.5"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "true < true"),
        ],
        &[
            ("operand-type", "true < true"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=((if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..]))\n",
        &[
            (
                "array-operand",
                "(if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])",
            ),
            (
                "type-mismatch",
                "(if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])",
            ),
            ("index-type", "[1, 2][[1, 2]..]"),
        ],
        &[("index-type", "[1, 2][[1, 2]..]")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])\n",
        &[
            (
                "array-operand",
                "(if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])",
            ),
            (
                "type-mismatch",
                "(if true then 1.5 else 300000000000) or ([1, 2][[1, 2]..])",
            ),
            ("index-type", "[1, 2][[1, 2]..]"),
        ],
        &[("index-type", "[1, 2][[1, 2]..]")],
    ),
    (
        "Clip c:\n    Note | velocity=((1.5 / 1) or (if [1, 2] then true else true))\n",
        &[
            (
                "type-mismatch",
                "(1.5 / 1) or (if [1, 2] then true else true)",
            ),
            ("type-mismatch", "1.5 / 1"),
            ("condition-type", "if [1, 2] then true else true"),
        ],
        &[("condition-type", "if [1, 2] then true else true")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (1.5 / 1) or (if [1, 2] then true else true)\n",
        &[
            (
                "type-mismatch",
                "(1.5 / 1) or (if [1, 2] then true else true)",
            ),
            ("type-mismatch", "1.5 / 1"),
            ("condition-type", "if [1, 2] then true else true"),
        ],
        &[("condition-type", "if [1, 2] then true else true")],
    ),
    (
        "Clip c:\n    Note | velocity=((if i32(true) then true else [1, 2][5])[300000000000 / q..])\n\nstate q(0): i32\n",
        &[
            ("type-mismatch", "if i32(true) then true else [1, 2][5]"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-type", "300000000000 / q"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-type", "300000000000 / q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=((if i32(true) then true else [1, 2][5])[300000000000 / q..])\n\nstate q(0): u8\n",
        &[
            ("type-mismatch", "if i32(true) then true else [1, 2][5]"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-type", "300000000000 / q"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-type", "300000000000 / q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < 300000000000) then (if [1, 2] then [1, 2] else 1.5) else (if q then [1, 2][5] else 300000000000))\n\nstate q(false)\n",
        &[
            ("condition-type", "if [1, 2] then [1, 2] else 1.5"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if [1, 2] then [1, 2] else 1.5"),
        ],
        &[
            ("condition-type", "if [1, 2] then [1, 2] else 1.5"),
            ("index-out-of-range", "5"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (300000000000 < 300000000000) then (if [1, 2] then [1, 2] else 1.5) else (if q then [1, 2][5] else 300000000000))\n\nlet q = false\n",
        &[
            ("condition-type", "if [1, 2] then [1, 2] else 1.5"),
            ("does-not-fit", "300000000000"),
            ("type-mismatch", "if [1, 2] then [1, 2] else 1.5"),
        ],
        &[
            ("condition-type", "if [1, 2] then [1, 2] else 1.5"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(if 300000000000 then q else q))\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(if 300000000000 then q else q))\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(if 300000000000 then q else q))\n\nlet q = [1]\n",
        &[
            ("array-operand", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(if 300000000000 then q else q)\n\nstate q(false)\n",
        &[
            ("reactive-let", "a"),
            ("conversion-type", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(if 300000000000 then q else q)\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(if 300000000000 then q else q)\n\nlet q = [1]\n",
        &[
            ("array-operand", "i32(if 300000000000 then q else q)"),
            ("condition-type", "300000000000"),
        ],
        &[("condition-type", "300000000000")],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(if 1.5 then q else true))\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else true)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(if 1.5 then q else true))\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else true)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(if 1.5 then q else true)\n\nstate q(false)\n",
        &[
            ("reactive-let", "a"),
            ("conversion-type", "i32(if 1.5 then q else true)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(if 1.5 then q else true)\n\nlet q = false\n",
        &[
            ("conversion-type", "i32(if 1.5 then q else true)"),
            ("condition-type", "1.5"),
        ],
        &[("condition-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (i32(true) div i32(true)) % (300000000000 * [1, 2][5])\n",
        &[
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("index-out-of-range", "5"),
        ],
        &[
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=((300000000000 % 1) div (if i32(true) then q else 300000000000))\n\nlet q = u8(1)\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (300000000000 % 1) div (if i32(true) then q else 300000000000)\n\nlet q = u8(1)\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("does-not-fit", "300000000000"),
        ],
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (if q then 300000000000 else i32(true))[q + q..]\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            ("not-an-array", "if q then 300000000000 else i32(true)"),
            ("operand-type", "q + q"),
            ("reactive-let", "a"),
        ],
        &[
            ("not-an-array", "if q then 300000000000 else i32(true)"),
            ("does-not-fit", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "q + q"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=((if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2]))\n",
        &[
            (
                "array-operand",
                "(if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])",
            ),
            (
                "type-mismatch",
                "(if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])",
            ),
            ("type-mismatch", "if [1, 2][5] then true else i32(true)"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "1.5"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "1.5"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = (if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])\n",
        &[
            (
                "array-operand",
                "(if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])",
            ),
            (
                "type-mismatch",
                "(if [1, 2][5] then true else i32(true)) is (if 1.5 then [1, 2] else [1, 2])",
            ),
            ("type-mismatch", "if [1, 2][5] then true else i32(true)"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "1.5"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "1.5"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = 1\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = u8(1)\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = i64(1)\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = f32(0.5)\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if 1 then q else 1) then (1.5 < i32(true)) else (if [1, 2] then 1 else true))\n\nlet q = 0.5\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if 1 then q else 1"),
            ("condition-type", "if [1, 2] then 1 else true"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2] then 1 else true"),
        ],
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "if [1, 2] then 1 else true"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0): i32\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0): u8\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0): i64\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0.5): f32\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1))\n\nstate q(0.5): f64\n",
        &[
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\n\nstate q(0): i32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\n\nstate q(0): u8\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\n\nstate q(0): i64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\n\nstate q(0.5): f32\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = if (if [1, 2][5] then i32(true) else q) then ([1, 2][5] div [1, 2]) else (true * 1)\n\nstate q(0.5): f64\n",
        &[
            ("reactive-let", "a"),
            ("condition-type", "if [1, 2][5] then i32(true) else q"),
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("index-out-of-range", "5"),
            ("operand-type", "true * 1"),
        ],
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("array-operand", "[1, 2][5] div [1, 2]"),
            ("operand-type", "true * 1"),
        ],
    ),
    (
        "Clip c:\n    Note | velocity=(i32(q[1.5..]))\n\nlet q = [1]\n",
        &[("array-operand", "i32(q[1.5..])"), ("index-type", "1.5")],
        &[("index-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = i32(q[1.5..])\n\nlet q = [1]\n",
        &[("array-operand", "i32(q[1.5..])"), ("index-type", "1.5")],
        &[("index-type", "1.5")],
    ),
    (
        "Clip c:\n    Note/(a)\nlet a: i32 = not (if q then i32(true) else 300000000000)\n\nstate q(false)\n",
        &[
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
            (
                "operand-type",
                "not (if q then i32(true) else 300000000000)",
            ),
            ("reactive-let", "a"),
            (
                "type-mismatch",
                "not (if q then i32(true) else 300000000000)",
            ),
        ],
        &[
            ("reactive-let", "a"),
            (
                "operand-type",
                "not (if q then i32(true) else 300000000000)",
            ),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
];

pub const CARRIED: &str = "480, carried to R4b";

pub fn reviewed(text: &str) -> Option<(&'static str, Vec<(String, String)>)> {
    let found = REVIEWED
        .iter()
        .find(|(_, program, _)| *program == text)
        .map(|(ledger, _, expected)| (*ledger, *expected))
        .or_else(|| {
            CARRIED_TO_R4B
                .iter()
                .find(|(program, ..)| *program == text)
                .map(|(_, new, _)| (CARRIED, *new))
        });
    found.map(|(ledger, expected)| {
        let mut expected: Vec<(String, String)> = expected
            .iter()
            .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
            .collect();
        expected.sort();
        (ledger, expected)
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
            "float-digits",
            "semicolon",
            "unterminated-string",
            "invalid-identifier",
            "invalid-escape",
            "unexpected-indent",
            "dedent-mismatch",
            "tab-indentation",
            "clause-column",
            "misplaced-caret",
            "misplaced-underscore",
            "misplaced-wire",
            "integer-too-large",
            "chained-comparison",
            "mixed-arguments",
            "match-position",
            "inline-placement",
            "top-level-wire",
            "unparenthesized-for",
            "unparenthesized-if",
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

fn path_beside_an_unknown_attribute(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Added
        && at.0 == "unknown-attribute"
        && seen.before(at).ends_with('.')
        && seen.old.iter().any(|drawn| {
            drawn.0 == "unknown-attribute"
                && drawn.2 < at.1
                && seen.program.get(drawn.2..at.1).is_some_and(|between| {
                    between.starts_with('=') && !between.contains(char::is_whitespace)
                })
        })
}

fn faulty_line(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let line = seen.line(at.1);
    let faulty = seen.old.iter().any(|drawn| {
        line_fault(&drawn.0)
            && seen.line(drawn.1) <= line
            && line <= seen.line(drawn.2.max(drawn.1 + 1) - 1)
    });
    if change != Change::Removed || !faulty {
        return false;
    }
    if at.0 == "reactive-let" {
        let nameless = (seen.before(at).is_empty() || seen.before(at).ends_with('\n'))
            && seen.text(at).starts_with("let ");
        return top_level_let_name(seen, at) || nameless;
    }
    [
        "array-operand",
        "operand-type",
        "type-mismatch",
        "not-an-array",
        "index-type",
        "does-not-fit",
        "plain-in-family",
    ]
    .contains(&at.0.as_str())
        && seen.line(at.2.saturating_sub(1).max(at.1)) == line
        && lines_at(seen.program)
            .get(line)
            .is_some_and(|(_, text)| text.starts_with("let "))
}

fn variable_inits(program: &str) -> Vec<(usize, usize, usize)> {
    lines_at(program)
        .into_iter()
        .filter_map(|(start, line)| {
            let word = ["state ", "stream ", "channel "]
                .into_iter()
                .find(|word| line.starts_with(word))?;
            let open = line.find('(')?;
            let close = line.rfind("): ")?;
            let ty = line.get(close + 3..)?;
            (open > word.len() && ty.contains("[..") && !line.contains(" = ")).then_some((
                start,
                start + open + 1,
                start + close,
            ))
        })
        .collect()
}

fn refused_variable_init(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change == Change::Restated {
        return false;
    }
    variable_inits(seen.program)
        .into_iter()
        .any(|(line, lo, hi)| {
            let refused = seen.old.iter().any(|drawn| {
                matches!(drawn.0.as_str(), "capacity-exceeded" | "type-mismatch")
                    && drawn.1 >= lo
                    && drawn.2 <= hi
            });
            if !refused {
                return false;
            }
            if seen.line(at.1) != seen.line(line) {
                return true;
            }
            change == Change::Removed
                && at.0 == "type-mismatch"
                && at.1 >= lo
                && at.2 <= hi
                && seen
                    .new
                    .iter()
                    .any(|drawn| drawn.1 >= at.1 && drawn.2 <= at.2 && drawn != at)
        })
}

fn float_literal(text: &str) -> bool {
    let digits = text
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '.')
        .collect::<String>();
    digits.starts_with(|c: char| c.is_ascii_digit())
        && (digits.contains('.') || digits.contains('e'))
        && !digits.ends_with("f32")
        && !digits.ends_with("f64")
}

fn integer_cell_in_a_flexible_float_operation(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    let reads = cell_reads(seen.program, name);
    change == Change::Added
        && at.0 == "integer-cell"
        && !reads.is_empty()
        && reads.iter().all(|(_, hi)| {
            let after = seen.program.get(*hi..).unwrap_or_default();
            [" + ", " - ", " * ", " / "]
                .iter()
                .any(|op| after.strip_prefix(op).is_some_and(float_literal))
        })
}

fn faulty_line_names<'a>(seen: &Seen<'a>) -> Vec<&'a str> {
    let mut names = Vec::new();
    let mut start = 0;
    for line in seen.program.split_inclusive('\n') {
        let end = start + line.len();
        let mut words = line.split_whitespace();
        if let (Some(kind), Some(rest)) = (words.next(), words.next())
            && !line.starts_with(' ')
            && !matches!(
                kind,
                "state" | "let" | "fn" | "pattern" | "stream" | "input"
            )
            && seen
                .old
                .iter()
                .any(|drawn| drawn.0 == "placement-syntax" && (start..end).contains(&drawn.1))
        {
            let name = rest
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .unwrap_or_default();
            if !name.is_empty() {
                names.push(name);
            }
        }
        start = end;
    }
    names
}

fn history_of_a_faulty_line_name(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    change == Change::Removed
        && at.0 == "history-target"
        && !name.is_empty()
        && faulty_line_names(seen).contains(&name)
}

fn history_of_a_faulty_let(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let text = seen.text(at);
    let Some((name, _)) = text.split_once('.') else {
        return false;
    };
    let head = format!("let {name} = ");
    change == Change::Added
        && at.0 == "history-target"
        && (text.ends_with(".stepped") || text.contains(".past("))
        && lines_at(seen.program).into_iter().any(|(start, line)| {
            let end = start + line.len();
            line.starts_with(&head)
                && seen.old.iter().any(|drawn| {
                    drawn.1 >= start + head.len() && drawn.2 <= end && drawn.0 != "reactive-let"
                })
                && !seen
                    .new
                    .iter()
                    .any(|drawn| drawn.0 == "reactive-let" && drawn.1 == start + 4)
        })
}

fn to_through_a_faulty_line_name(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let names = faulty_line_names(seen);
    change == Change::Removed
        && matches!(at.0.as_str(), "to-candidates" | "unknown-attribute")
        && seen.program.lines().any(|line| {
            line.contains("to.")
                && line.split_once(": ").is_some_and(|(_, candidates)| {
                    candidates
                        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .any(|word| names.contains(&word))
                })
        })
}

fn not_a_value_in_an_unexpanded_loop_body(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let before = seen.program.get(..at.1).unwrap_or_default();
    let line = before.rsplit('\n').next().unwrap_or_default();
    let depth = line.len() - line.trim_start().len();
    let in_loop = before.lines().rev().skip(1).any(|earlier| {
        let indent = earlier.len() - earlier.trim_start().len();
        indent < depth && earlier.trim_start().starts_with("for ")
    });
    let unexpanded = seen.old.iter().any(|drawn| drawn.0 == "unknown-name");
    change == Change::Added && at.0 == "not-a-value" && in_loop && unexpanded
}

fn loop_name_in_an_unknown_kinds_body(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    let names_a_loop_variable = [
        format!("for {name} in "),
        format!("for {name}, "),
        format!(", {name} in "),
    ]
    .iter()
    .any(|head| seen.program.contains(head.as_str()));
    let unknown_parent = seen.old.iter().any(|drawn| {
        drawn.0 == "unknown-kind"
            && drawn.1 < at.1
            && seen.program.get(drawn.2..at.1).is_some_and(|between| {
                between
                    .trim_start()
                    .strip_prefix(':')
                    .is_some_and(|rest| rest.trim().is_empty())
            })
    });
    change == Change::Added
        && at.0 == "unknown-kind"
        && !name.is_empty()
        && names_a_loop_variable
        && unknown_parent
}

fn fit_beside_a_name_that_is_no_value(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Removed
        && matches!(at.0.as_str(), "does-not-fit" | "type-mismatch")
        && seen.new.iter().any(|drawn| {
            drawn.0 == "not-a-value" && seen.line(drawn.1) == seen.line(at.1) && drawn.1 < at.1
        })
}

fn destination_of_a_refused_annotated_let(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    change == Change::Added
        && at.0 == "destination-type"
        && !name.is_empty()
        && lines_at(seen.program).iter().any(|(start, line)| {
            line.strip_prefix("let ")
                .and_then(|rest| rest.strip_prefix(name))
                .is_some_and(|rest| rest.starts_with(':'))
                && seen.new.iter().any(|drawn| {
                    drawn.0 == "type-mismatch" && seen.line(drawn.1) == seen.line(*start)
                })
        })
}

fn integer_cell_as_a_float_start(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    change == Change::Added
        && at.0 == "integer-cell"
        && !name.is_empty()
        && (seen.program.contains(&format!("from {name}:"))
            || seen.program.contains(&format!("from [{name}]:")))
}

fn reactive_let_history_target(seen: &Seen<'_>, at: &Drawn) -> bool {
    let text = seen.text(at);
    let Some((name, _)) = text.split_once('.') else {
        return false;
    };
    at.0 == "history-target"
        && (text.ends_with(".stepped") || text.contains(".past("))
        && seen.new.iter().any(|drawn| {
            drawn.0 == "reactive-let" && top_level_let_name(seen, drawn) && seen.text(drawn) == name
        })
}

fn history_cells_of_lets<'a>(seen: &Seen<'a>) -> Vec<&'a str> {
    let targets: Vec<&str> = seen
        .new
        .iter()
        .filter(|drawn| reactive_let_history_target(seen, drawn))
        .filter_map(|drawn| seen.text(drawn).split_once('.').map(|(name, _)| name))
        .collect();
    lines_at(seen.program)
        .into_iter()
        .filter_map(|(_, line)| {
            let (cell, rule) = line.strip_prefix("state ")?.split_once(" = ")?;
            let (target, _) = rule.split_once(".past(")?;
            targets.contains(&target).then_some(cell)
        })
        .collect()
}

fn read_of_a_lets_history_cell(seen: &Seen<'_>, at: &Drawn) -> bool {
    let line = lines_at(seen.program)
        .into_iter()
        .rfind(|(start, _)| *start <= at.1)
        .map(|(_, line)| line)
        .unwrap_or_default();
    matches!(
        at.0.as_str(),
        "float-to-integer" | "does-not-fit" | "condition-type" | "operand-type"
    ) && !line.starts_with("state ")
        && history_cells_of_lets(seen)
            .iter()
            .any(|cell| reads_word(line, cell))
}

fn history_of_a_reactive_let(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    match change {
        Change::Added => {
            reactive_let_history_target(seen, at)
                || (matches!(
                    at.0.as_str(),
                    "float-to-integer"
                        | "type-mismatch"
                        | "does-not-fit"
                        | "condition-type"
                        | "operand-type"
                        | "index-out-of-range"
                ) && seen
                    .new
                    .iter()
                    .any(|drawn| reactive_let_history_target(seen, drawn)))
        }
        Change::Removed => read_of_a_lets_history_cell(seen, at),
        Change::Restated => false,
    }
}

fn host_fed_init(seen: &Seen<'_>, at: &Drawn) -> bool {
    let line = seen.before(at).rsplit('\n').next().unwrap_or_default();
    at.0 == "reactive-initializer" && line.starts_with("state ") && line.ends_with('(')
}

fn history_init(seen: &Seen<'_>, at: &Drawn) -> bool {
    host_fed_init(seen, at)
        && (seen.text(at).contains(".past(") || seen.text(at).contains(".stepped"))
}

fn cycle_through_history_gone(seen: &Seen<'_>) -> bool {
    seen.old.iter().any(|drawn| drawn.0 == "declaration-cycle")
        && !seen.new.iter().any(|drawn| drawn.0 == "declaration-cycle")
        && seen.new.iter().any(|drawn| history_init(seen, drawn))
}

fn host_fed_init_reading_history(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    match change {
        Change::Added => host_fed_init(seen, at) && cycle_through_history_gone(seen),
        Change::Removed => {
            let name = seen.text(at);
            at.0 == "declaration-cycle"
                && seen.program.contains(&format!("state {name}("))
                && cycle_through_history_gone(seen)
        }
        Change::Restated => false,
    }
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
    if change == Change::Removed
        && at.0 == "does-not-fit"
        && beside_a_fault(seen)
        && seen.new.iter().any(|drawn| {
            drawn.0 == "not-an-array"
                && drawn.1 <= at.1
                && at.2 <= drawn.2
                && indexed_base(seen, drawn)
        })
    {
        return true;
    }
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

fn array_cells(program: &str) -> Vec<&str> {
    program
        .lines()
        .filter_map(|line| line.strip_prefix("state ")?.split_once(" = "))
        .filter(|(name, rule)| !name.contains(':') && rule.starts_with('['))
        .map(|(name, _)| name)
        .chain(variable_array_cells(program))
        .collect()
}

fn annotated_array_rules(program: &str) -> Vec<(&str, usize, usize)> {
    let arrays = array_cells(program);
    let mut rules = Vec::new();
    let mut start = 0;
    for line in program.split_inclusive('\n') {
        let text = line.trim_end_matches('\n');
        if let Some((head, rule)) = text
            .strip_prefix("state ")
            .and_then(|tail| tail.split_once(" = "))
            && let Some((name, ty)) = head.split_once(": ")
            && !ty.is_empty()
            && (rule.starts_with('[') || arrays.iter().any(|array| reads_word(rule, array)))
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
    let code = at.0.as_str();
    if change == Change::Added && code == "capacity-exceeded" {
        let text = seen.text(at);
        return text.starts_with("for ") && annotated_accumulator(text) && text.contains("[..");
    }
    if change != Change::Added
        || !matches!(
            code,
            "type-mismatch" | "float-to-integer" | "does-not-fit" | "integer-cell"
        )
    {
        return false;
    }
    if code == "integer-cell" {
        let name = seen.text(at);
        return seen.before(at).ends_with("state ")
            && seen.program.lines().any(|line| {
                annotated_accumulator(line)
                    && line
                        .split_once(" from ")
                        .and_then(|(_, rest)| rest.split_once(": "))
                        .is_some_and(|(start, _)| reads_word(start, name))
            });
    }
    let line = seen
        .program
        .lines()
        .nth(seen.line(at.1))
        .unwrap_or_default();
    let before = seen.before(at);
    let head = before.rsplit('\n').next().unwrap_or_default();
    let loop_name = code == "type-mismatch"
        && seen.after(at).starts_with(": ")
        && (head.ends_with("for ") || head.ends_with(", "))
        && line.contains(" in ");
    let inside = annotated_accumulator(line)
        && head
            .rsplit_once(" into ")
            .is_some_and(|(_, rest)| rest.contains(" from "));
    loop_name || inside
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

fn integer_cell_read_beside_a_refusal(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    let refused = |lo: usize, hi: usize| {
        seen.new.iter().any(|drawn| {
            matches!(
                drawn.0.as_str(),
                "type-mismatch" | "operand-type" | "index-type" | "condition-type"
            ) && drawn.1 <= lo
                && hi <= drawn.2
        })
    };
    change == Change::Added
        && at.0 == "integer-cell"
        && cell_reads(seen.program, name).iter().any(|(lo, hi)| {
            refused(*lo, *hi)
                || calls_of_the_body_at(seen.program, *lo)
                    .iter()
                    .any(|(start, end)| refused(*start, *end))
        })
}

fn calls_of_the_body_at(program: &str, at: usize) -> Vec<(usize, usize)> {
    let Some((start, line)) = lines_at(program)
        .into_iter()
        .rfind(|(start, _)| *start <= at)
    else {
        return Vec::new();
    };
    let Some(name) = line
        .strip_prefix("fn ")
        .and_then(|rest| rest.split_once('('))
        .map(|(name, _)| name)
        .filter(|name| !name.is_empty())
    else {
        return Vec::new();
    };
    let call = format!("{name}(");
    program
        .match_indices(&call)
        .filter(|(found, _)| {
            word_start(program, *found) && !(start..start + line.len()).contains(found)
        })
        .map(|(found, _)| (found, found + call.len()))
        .collect()
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

struct Choice {
    span: (usize, usize),
    condition: (usize, usize),
    arms: [(usize, usize); 2],
}

fn choice_at(program: &str, start: usize) -> Option<Choice> {
    let mut depth = 0usize;
    let (mut then, mut otherwise, mut end) = (None, None, program.len());
    for (offset, character) in program.get(start..)?.char_indices() {
        let at = start + offset;
        let rest = program.get(at..).unwrap_or_default();
        match character {
            '(' | '[' => depth += 1,
            ')' | ']' if depth == 0 => {
                end = at;
                break;
            }
            ')' | ']' => depth -= 1,
            '\n' if depth == 0 => {
                end = at;
                break;
            }
            '.' if depth == 0 && otherwise.is_some() && rest.starts_with("..") => {
                end = at;
                break;
            }
            _ if depth == 0 && then.is_none() && rest.starts_with(" then ") => then = Some(at),
            _ if depth == 0
                && then.is_some()
                && otherwise.is_none()
                && rest.starts_with(" else ") =>
            {
                otherwise = Some(at);
            }
            _ => {}
        }
    }
    let (then, otherwise) = (then?, otherwise?);
    Some(Choice {
        span: (start, end),
        condition: (start + 3, then),
        arms: [(then + 6, otherwise), (otherwise + 6, end)],
    })
}

fn choices(program: &str) -> Vec<Choice> {
    program
        .match_indices("if ")
        .filter(|(start, _)| word_start(program, *start))
        .filter_map(|(start, _)| choice_at(program, start))
        .collect()
}

fn unwrapped(program: &str, (start, end): (usize, usize)) -> (usize, usize) {
    match program.get(start..end) {
        Some(text) if text.starts_with('(') && closing(program, start) == Some(end - 1) => {
            (start + 1, end - 1)
        }
        _ => (start, end),
    }
}

fn condition_refused(seen: &Seen<'_>, choice: &Choice) -> bool {
    seen.new.iter().any(|drawn| {
        (drawn.1 >= choice.condition.0 && drawn.2 <= choice.condition.1)
            || (drawn.0 == "condition-type" && (drawn.1, drawn.2) == choice.span)
    })
}

fn choice_keeps_its_arms_type(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if !beside_a_fault(seen) {
        return false;
    }
    let refused: Vec<Choice> = choices(seen.program)
        .into_iter()
        .filter(|choice| condition_refused(seen, choice))
        .collect();
    match change {
        Change::Added => {
            matches!(
                at.0.as_str(),
                "type-mismatch"
                    | "operand-type"
                    | "index-type"
                    | "not-an-array"
                    | "does-not-fit"
                    | "condition-type"
                    | "conversion-type"
                    | "array-operand"
            ) && refused.iter().any(|choice| {
                (at.1 <= choice.span.0 && choice.span.1 <= at.2)
                    || seen.program.get(choice.span.1..at.1) == Some(")[")
            })
        }
        Change::Removed => refused.iter().any(|choice| match at.0.as_str() {
            "type-mismatch" => choice
                .arms
                .iter()
                .any(|arm| unwrapped(seen.program, *arm) == (at.1, at.2)),
            "index-type" => {
                seen.program.get(choice.span.1..at.1) == Some(")[")
                    && seen
                        .new
                        .iter()
                        .any(|drawn| drawn.0 == "not-an-array" && (drawn.1, drawn.2) == choice.span)
            }
            "does-not-fit" => seen.new.iter().any(|drawn| {
                drawn.1 <= at.1
                    && at.2 <= drawn.2
                    && drawn.1 <= choice.span.0
                    && choice.span.1 <= drawn.2
            }),
            _ => false,
        }),
        Change::Restated => false,
    }
}

fn cell_type<'a>(program: &'a str, name: &str) -> Option<&'a str> {
    program.lines().find_map(|line| {
        line.strip_prefix("state ")
            .and_then(|tail| tail.strip_prefix(name))
            .filter(|tail| tail.starts_with('('))
            .and_then(|tail| tail.rsplit_once("): "))
            .map(|(_, ty)| ty.trim())
    })
}

fn choice_arm_in_the_cells_type(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let text = seen.text(at);
    let beside_the_cell = choices(seen.program).iter().any(|choice| {
        let [first, second] = choice.arms.map(|arm| unwrapped(seen.program, arm));
        let read = |(start, end): (usize, usize)| seen.program.get(start..end) == Some("q");
        (first == (at.1, at.2) && read(second)) || (second == (at.1, at.2) && read(first))
    });
    let ty = cell_type(seen.program, "q").unwrap_or_default();
    at.0 == "does-not-fit"
        && beside_a_fault(seen)
        && !text.is_empty()
        && text.chars().all(|character| character.is_ascii_digit())
        && beside_the_cell
        && match change {
            Change::Added => ["u8", "i32", "i64"].contains(&ty),
            Change::Removed => ty == "f64",
            Change::Restated => false,
        }
}

fn annotated_placement_loop_name(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if at.0 != "type-mismatch" {
        return false;
    }
    let head = seen.before(at).rsplit('\n').next().unwrap_or_default();
    let rest = seen.after(at).split('\n').next().unwrap_or_default();
    let line = format!("{head}{}{rest}", seen.text(at));
    let names = line
        .trim_start()
        .strip_prefix("for ")
        .and_then(|tail| tail.split_once(" in "))
        .map(|(names, _)| names);
    let Some(names) = names else {
        return false;
    };
    if !line.starts_with(' ') || line.contains(" into ") || !names.contains(": ") {
        return false;
    }
    match change {
        Change::Added | Change::Restated => {
            seen.after(at).starts_with(": ") && (head.ends_with("for ") || head.ends_with(", "))
        }
        Change::Removed => head.ends_with(" in ") && rest == ":",
    }
}

fn inline_order_in_a_loop_head(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let line = seen.before(at).rsplit('\n').next().unwrap_or_default();
    let rest = seen.after(at).split('\n').next().unwrap_or_default();
    let in_a_head = line.trim_start().starts_with("for ")
        && line.contains(" in ")
        && !line.contains(" into ")
        && !rest.contains(" into ");
    let call = seen.text(at).ends_with(')') && seen.text(at).contains('(');
    let count = |set: &[Drawn]| set.iter().filter(|drawn| drawn.0 == at.0).count();
    change != Change::Restated
        && matches!(at.0.as_str(), "inline-limit" | "limit-warning")
        && in_a_head
        && call
        && count(seen.old) == count(seen.new)
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
        rule: "a let declared on a line holding a syntax fault is a placeholder and its value draws nothing of its own, its reactive-let and a type fault on its line alike, since the line's intended text is unknown (ARCHITECTURE §7: a syntax fault leaves no type known)",
        explains: faulty_line,
    },
    Class {
        ledger: "461",
        rule: "a history read with neither fallback nor init in a group of cells yields the cell's own settled shape, the group's open kinds taking their inits' (SPEC §6.6), where the old engine joins it as a single value and draws array-operand and reads the cell as a placeholder, so an annotation reading it draws the type-mismatch its settled shape gives",
        explains: open_history_join,
    },
    Class {
        ledger: "476",
        rule: "a top-level let annotated with a variable array of a scalar draws an element that does not have or fit the element type at the element (ARCHITECTURE §7: every fault draws at its span), as a let annotated with a fixed array does on both engines, where the old engine draws it at the whole declaration",
        explains: variable_let_element,
    },
    Class {
        ledger: "476",
        rule: "a top-level let whose value its array annotation refuses, by its rows or its capacity, keeps the annotated type (ARCHITECTURE §7: a value whose type an error leaves known keeps that type), so a read of it draws the faults that type gives, where the old engine reads the let as a placeholder",
        explains: refused_array_let_keeps_its_type,
    },
    Class {
        ledger: "476",
        rule: "an unclocked cycle of cells whose history reads have neither fallback nor init and that reads no cell outside itself draws unrun-cell at its first cell (SPEC §6.6: the same error as a cell that reads only its own history), where the old engine joins the open reads as single values and draws not-an-array at the slice over them",
        explains: unrun_cycle,
    },
    Class {
        ledger: "476",
        rule: "a host-fed cell annotated with a variable array whose init the cell refuses draws one diagnostic for the fault, at each refused element or at the init, and its reads keep the annotated type (ARCHITECTURE §7: a value whose type an error leaves known keeps that type), where the old engine adds a type-mismatch at the whole init and reads the cell as a placeholder",
        explains: refused_variable_init,
    },
    Class {
        ledger: "476",
        rule: "an integer host-fed cell read only as the start of a `for` expression whose accumulator settles to a float draws integer-cell (SPEC §4.4), every use expecting a float",
        explains: integer_cell_as_a_float_start,
    },
    Class {
        ledger: "476",
        rule: "an integer host-fed cell read only as the operand of an operation with a flexible float literal is read as a float there (SPEC §4.3: a flexible float meeting an operand of fixed integer type makes the operation a float one, `f64` for an `i64` operand), so every use expects a float and the cell draws integer-cell (SPEC §4.4), also where the operation stands in a comparison, where the old engine draws nothing (`state big(1)` with `Note when big + 1e39 > 1`, `big + 1.5 > 1` or `big * 2.5 > 1`)",
        explains: integer_cell_in_a_flexible_float_operation,
    },
    Class {
        ledger: "476",
        rule: "a placement in the body of a placement of an unknown kind, inside a placement `for` body, whose kind is a loop variable's name draws unknown-kind, as it does under a known kind and in the program with the unknown kind repaired (the repair rule; SPEC §7.1), where the old engine is silent (`for s in sections:` with `xStarts:` holding `s` on the next line)",
        explains: loop_name_in_an_unknown_kinds_body,
    },
    Class {
        ledger: "476",
        rule: "a history read of a name declared by a placement on a line holding a syntax fault draws nothing, the name being a placeholder wherever it is used (ARCHITECTURE §7), where the old engine draws history-target at it (`xstate gate(false)` with `gate.past(2)`)",
        explains: history_of_a_faulty_line_name,
    },
    Class {
        ledger: "476",
        rule: "a history read of a top-level let whose value drew a fault draws history-target, as the program with the value repaired to a valid compile-time value does (DECISIONS.md 2026-11-03: a diagnostic stands beside another fault exactly when the repaired program still draws it; a let has no history whatever its value holds), where the old engine draws nothing at the read (`let t = zzz` with `t.past(1, [])`)",
        explains: history_of_a_faulty_let,
    },
    Class {
        ledger: "476",
        rule: "a cell rule's `for` expression draws range-bound at a range bound that reads a cell with no maximum beside a fault later on its line, in its head, its start or its body, as the program with that fault repaired does (DECISIONS.md 2026-11-03: a diagnostic stands beside another fault exactly when the repaired program still draws it), where the old engine draws nothing at the bound (`state total = for i in 0..n into sum from 0: sum + zz`)",
        explains: range_bound_beside_a_faulty_fold,
    },
    Class {
        ledger: "476",
        rule: "a read of a scalar-annotated cell beside the cell's refused init keeps the cell's type, so its member draws unknown-attribute and its join with an array array-operand, as the program with the init repaired to a valid value of the cell's type does (DECISIONS.md 2026-11-03), where the old engine draws nothing at the read (`state held([]): i32` with `held.size` or `held + [note]`)",
        explains: read_of_a_cell_beside_its_refused_init,
    },
    Class {
        ledger: "476",
        rule: "a slice whose bound holds a syntax fault is still an array, so a position of a single value draws type-mismatch at it, as the program with the bound repaired does (DECISIONS.md 2026-11-03), where the old engine draws nothing at the slice (`Note/(xs[1 % ..3])`)",
        explains: slice_with_a_faulty_bound,
    },
    Class {
        ledger: "476",
        rule: "a `to` path whose candidate list names an instance declared by a placement on a line holding a syntax fault draws nothing of that candidate, the name being a placeholder wherever it is used (ARCHITECTURE §7), where the old engine reads the instance and draws to-candidates or unknown-attribute at the path (`Clip chorus(:` with `gain=to.hook.velocity: [chorus, bridge][section]`)",
        explains: to_through_a_faulty_line_name,
    },
    Class {
        ledger: "476",
        rule: "a vocabulary name standing as a value in the body of a placement `for` whose iterable holds a placeholder draws not-a-value, a name resolution error, as code nothing expands draws its name resolution errors (SPEC §6.5; `let u = sin` draws it unread on both engines), where the old engine is silent in such a body (`for s in sections:` with `when Starts: s`)",
        explains: not_a_value_in_an_unexpanded_loop_body,
    },
    Class {
        ledger: "480",
        rule: "a vocabulary name standing as a value (a node kind, a wire kind, an enum, a function) draws not-a-value and is a placeholder, so a fault at a literal beside it that holds only for some value of the name, a fit or a float in an integer position, does not stand (the placeholder rule, as beside an unknown name), where the old engine checks the literal in the position's type (`Note/(Lamp + 300000000000)`)",
        explains: fit_beside_a_name_that_is_no_value,
    },
    Class {
        ledger: "476",
        rule: "a let annotated with a value type whose init it refuses keeps the annotated type (ARCHITECTURE §7: a value whose type an error leaves known keeps that type), so a destination naming it draws destination-type, as the program with the init repaired to a value of that type does, where the old engine reads the let as a placeholder (`let d: i32 = a` with `Plays: d`)",
        explains: destination_of_a_refused_annotated_let,
    },
    Class {
        ledger: "487",
        rule: "a history read of a top-level let reading a cell draws history-target at the read beside the let's reactive-let, as the program with the let repaired to a compile-time value does, and the read of an annotated let keeps the annotated type (ARCHITECTURE §7: a value whose type an error leaves known keeps that type), so its readers draw the faults that type gives whatever value repairs the read, where the old engine reads it as a placeholder; the read of an unannotated top-level let is typed as the cell the let repaired to a `state` would be (ledger 516), a float one read as an integer drawing float-to-integer, an array one's index its range, and a fallback the repaired cell refuses its type-mismatch; a fault the old engine drew at a read of a cell whose rule is that history read is removed only where the program with the let repaired to a `state` draws none there (`Note when s > 3000000000`, `Note when s` and `not s` beside a `0` fallback of a `bool` let, `Note/(s)` beside a `0.5` fallback of an integer let reading a cell); a compile-time let is no repaired cell, so its history read stays a placeholder that draws nothing further",
        explains: history_of_a_reactive_let,
    },
    Class {
        ledger: "476",
        rule: "a variable-array cell standing as a choice's condition draws one condition-type, at the choice, as its fixed-array twin does (ARCHITECTURE §7: one fault draws one diagnostic), where the old engine draws a second one at the condition",
        explains: variable_array_condition,
    },
    Class {
        ledger: "476",
        rule: "an operator over a choice whose arms hold a single value and a variable-array cell is a placeholder, so it draws no type-mismatch at its position, as over the fixed-array twin on both engines (the repair rule: an array arm makes the operator array-operand), where the old engine draws one for the variable twin",
        explains: operator_over_a_variable_shape_choice,
    },
    Class {
        ledger: "476",
        rule: "the arms of a choice whose condition is refused are checked as a runtime choice's (DECISIONS.md 11:07Z (1)), with or without a variable-array cell, so their index ranges, fits and types draw inside them, the arms unify as a runtime choice's (a type-mismatch at the choice where they disagree), and a cell read in them is a use, which draws integer-cell where every use expects a float and keeps it from drawing where the choice stands as a condition, as the program with the condition repaired to a cell's read draws them, where the old engine is silent",
        explains: arm_of_a_refused_condition,
    },
    Class {
        ledger: "476",
        rule: "beside a variable-array cell, the arms of a choice whose condition is refused are checked as a runtime choice's (DECISIONS.md 11:07Z (1)) and an operand that a refused operator, index, bound or conversion holds is checked in its own type (ARCHITECTURE §7), so their index ranges, types and array operands draw there, where the old engine's variable-array path is silent",
        explains: refused_beside_a_variable_array,
    },
    Class {
        ledger: "476",
        rule: "a read indexed or sliced by a variable-array cell draws what its fixed-array twin draws on both engines: index-type alone at an element read of an array, and index-type beside not-an-array at a single-value base (ledger 497), where the old engine's variable path draws not-an-array in place of the first and alone in the second",
        explains: variable_array_index,
    },
    Class {
        ledger: "489",
        rule: "a host-fed cell whose init reads a cell's history, its own or another host-fed cell's, draws reactive-initializer at the init (SPEC §6.1: the init is a compile-time value) and no declaration-cycle (SPEC §6.6: a history read forms no cycle), where the old engine draws declaration-cycle through the history read",
        explains: host_fed_init_reading_history,
    },
    Class {
        ledger: "476",
        rule: "condition-type at a choice's non-bool condition beside its mismatched or placeholder arms, in place of the old engine's type-mismatch there",
        explains: condition_beside_arms,
    },
    Class {
        ledger: "476",
        rule: "not-an-array at an indexed or sliced base that is not an array, in place of the old engine's type-mismatch there, and no does-not-fit inside the base, which is checked in its own type (ARCHITECTURE §7: an operand that a refused index or bound holds is checked in its own type), as the program with the bound repaired draws",
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
        rule: "an annotated fold accumulator's start and body, and an annotated loop name's elements, draw type-mismatch where they do not have the annotated type (SPEC §4.5), and the start and body of an annotated fold draw what the same expression draws as the rule of a derived cell with that annotation (SPEC §5.10, §4.2; DECISIONS.md 22:13Z), which the old engine misses; and a compile-time `for` expression whose accumulator is annotated `T[..N]` draws capacity-exceeded where the capacity its body's rule finds over the iterations its iterable can run passes N (SPEC §5.10, ledger 502), as its reactive twin does on both engines, where the old engine is silent",
        explains: annotated_fold,
    },
    Class {
        ledger: "476",
        rule: "a top-level let with an array annotation fixes its whole value's type: each element that does not have or fit the element type draws at itself whichever element a read selects, and an element read keeps the fixed element type, which the old engine misses",
        explains: annotated_array_let,
    },
    Class {
        ledger: "476",
        rule: "a path in an unknown attribute's value draws its own unknown-attribute at the segment that names no child, wire or attribute, since the program with the attribute repaired to one the kind declares still draws it, which the old engine misses",
        explains: path_beside_an_unknown_attribute,
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
        rule: "an integer host-fed cell read inside an operator, index, slice or condition that draws a refusal draws integer-cell when the program with each refused value repaired reads the cell only where a float is expected (SPEC §4.4: every use expects a float), where the old engine counts the read inside the refusal as a use that expects no float",
        explains: integer_cell_read_beside_a_refusal,
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
    Class {
        ledger: "476",
        rule: "a choice keeps its arms' type whatever its condition holds (G18, ARCHITECTURE §7: a value whose type an error leaves known keeps that type), so beside a refused condition the faults that type gives draw at the choice, at the expression or condition holding it and at a bound beside it, a kept scalar base draws not-an-array in place of its bound's index-type (SPEC §5.5), and the old engine's type-mismatch at one arm, which only that arm taken draws, does not, nor its does-not-fit at a literal beside the choice inside an expression the new core refuses whole, which it computes in its `f32` default for the choice",
        explains: choice_keeps_its_arms_type,
    },
    Class {
        ledger: "476",
        rule: "a literal arm beside an annotated cell's read is checked in the choice's own type, the cell's (ARCHITECTURE §7's own-type sentence), so it draws does-not-fit against an integer cell's type, which the old engine misses, and none beside an f64 cell, where the old engine checks it against f32",
        explains: choice_arm_in_the_cells_type,
    },
    Class {
        ledger: LOOP_HEAD_STOP,
        rule: "a loop head's calls are inlined in source order, left to right (ARCHITECTURE §7 stage 6: a frame evaluates an operator's operands in source order), so its limit-warning and inline-limit fall on the calls that order reaches past the warning value and the limit, where the old engine counts right to left",
        explains: inline_order_in_a_loop_head,
    },
    Class {
        ledger: "476",
        rule: "a `for` expression iterating a cell whose init its scalar annotation refuses draws iterable-type at the iterable, since the refused init leaves the annotated scalar type known (ARCHITECTURE §7) and a `for` iterates a range or an array (SPEC §7.9), where the old engine reads the cell as a placeholder",
        explains: iterable_of_a_refused_init,
    },
    Class {
        ledger: "476",
        rule: "an annotated name of a placement `for` fixes its type as a `for` expression's does (SPEC §7.9, §4.5), so elements that do not have the annotated type draw one type-mismatch at the name with the loop name's summary, where the old engine draws it at the iterable or with a value's summary",
        explains: annotated_placement_loop_name,
    },
    Class {
        ledger: "476",
        rule: "an integer host-fed cell read only in an arm a compile-time choice does not take, directly or through a function or a `let` read only there, has no use there (SPEC §6.2: such a choice reads the cells of the arm it takes alone; ARCHITECTURE §7: an untaken arm is checked in its own types only) and draws no integer-cell, where the old engine counts the untaken arm's read as a float use",
        explains: integer_cell_read_only_untaken,
    },
    Class {
        ledger: "476",
        rule: "a function body's local `let` that holds a cell's read stands for that read in the body, checked as inlined at its call (SPEC §6.3), so the cell read through the `let` where a float is expected is a float use and draws integer-cell, as the same body reading the cell directly draws on both engines, where the old engine misses it through the `let`",
        explains: integer_cell_through_a_local_let,
    },
    Class {
        ledger: "476",
        rule: "a condition the parser ends where a syntax fault on its line begins leaves the intended condition unknown, the rest of it lying in that fault's region (ARCHITECTURE §7.1: a fault's region is the rest of its line), so the choice draws no condition-type, where the old engine types the cut condition as written (the head's reading of 15:39Z 2026-11-05)",
        explains: condition_cut_by_a_line_fault,
    },
    Class {
        ledger: "476",
        rule: "a gate whose value holds a refused index, slice bound, conversion or condition draws the faults the type those faults leave known gives, condition-type at the value where it is no `bool`, type-mismatch or array-operand at an operand of `and` or `or` (ARCHITECTURE §7: a value whose type an error leaves known keeps that type), as the program with the faults repaired draws on both engines, where the old engine is silent; and a gate that is an array choice draws one condition-type, where the old engine draws a second at the same span; a gate value refused as no `bool` is checked in its own type, so a literal inside it that does not fit draws does-not-fit (ARCHITECTURE §7: an operand that a refused condition holds is checked in its own type), where the old engine checks nothing inside it, a negated choice whose arms unify in `u8` saturating there draws constant-overflow (SPEC §4.6); and the old engine's condition-type at a slice condition whose bound holds a refused index, which the program with the index repaired draws on neither engine, does not draw",
        explains: gate_beside_its_faults,
    },
    Class {
        ledger: "476",
        rule: "a derived cell's rule or a reactive pattern argument whose operand is a conversion of a cell to `u8` or `i32` computes in that fixed type where no integer use gives it a width, so a literal beside it that does not fit that type draws does-not-fit at the literal (SPEC §4.1: the fit check applies where a fixed-type operand gives a literal its type; §4.4 and §4.5: an integer derived cell's width is the widest its integer uses expect, else the widest of its operands), as the comparison with such a cell gives a literal the cell's type, where the old engine is silent or draws it only for some inits of the cell; such an argument read at a float use is the expression itself at that use (SPEC §6.6, ledger 483), the conversion widening into the float type before the operation (SPEC §4.2), so the literal fits, where the old engine draws does-not-fit in the conversion's type for some inits",
        explains: fixed_type_operand_of_a_cell_rule,
    },
    Class {
        ledger: "476",
        rule: "a member call in the value of a let nothing uses or in the body of a function nothing calls draws nothing, since code nothing uses has its names resolved and nothing else checked (SPEC §6.5) and whether the member names anything depends on its receiver's type, where the old engine draws not-callable or unknown-name at it in some programs and not in others (`let xs = [1, 2]` with `let n = xs.size()` and no placement reading `n`, or `let n = [1, 2][0].keys()`)",
        explains: member_call_in_an_unused_value,
    },
    Class {
        ledger: "476",
        rule: "a placement or a wire in the body of a placement `for` whose head holds a fault draws its unknown attribute, flag or parameter names, as the program with the head repaired to any valid iterable does, a head of no iteration included (the repair rule, DECISIONS.md 2026-11-03), where the old engine is silent in such a body (`for i in zz:` with `Lamp | foo=1` or `Lamp~`)",
        explains: member_name_in_an_unexpanded_loop_body,
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

fn bound_beside_a_kept_choice(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Added
        && at.0 == "index-type"
        && choices(seen.program).iter().any(|choice| {
            condition_refused(seen, choice)
                && seen.program.get(choice.span.1..at.1) == Some(")[")
                && choice
                    .arms
                    .iter()
                    .any(|arm| seen.program.get(arm.0..arm.1) == Some("q"))
        })
}

fn gate_beside_the_twin_pass(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let start = seen
        .program
        .get(..at.1)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |at| at + 1);
    let end = seen
        .program
        .get(at.2..)
        .and_then(|after| after.find('\n'))
        .map_or(seen.program.len(), |found| at.2 + found);
    let line = seen.program.get(start..end).unwrap_or_default();
    let gate = line.find(" when ").map(|found| start + found);
    change != Change::Restated
        && gate.is_some_and(|gate| gate < at.1)
        && seen.program.get(at.1..end).is_some_and(|rest| {
            rest.contains('q')
                || seen
                    .program
                    .get(start..at.1)
                    .is_some_and(|head| head.contains('q'))
        })
}

pub const TWIN_PASS_CLASSES: &[Class] = &[
    Class {
        ledger: "476, the twin pass's (R4b)",
        rule: "a type-mismatch at an annotated let's value that every value of its placeholder draws, which the twin pass misses because its `state q(false)` twin runs on the old engine and draws reactive-let alone there",
        explains: annotation_beside_the_twin_pass,
    },
    Class {
        ledger: "476, the twin pass's (R4b)",
        rule: "index-type at the bound or index of a read whose base is a choice over the placeholder with a refused condition, which stands since the repaired bound still draws it (G18), and which the twin pass misses because its scalar twins keep the choice's type and draw not-an-array at the base in its place",
        explains: bound_beside_a_kept_choice,
    },
    Class {
        ledger: "476, the twin pass's (R4b)",
        rule: "at a gate holding a name placeholder, the twin pass runs the whole-program twins on the old engine, so the gate draws the old engine's set (a conversion's target type, a refused condition, a choice's arms, a sliced base, an index or a fit beside the placeholder), where the new core's twins differ (488's and 497's kind)",
        explains: gate_beside_the_twin_pass,
    },
];

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

fn lines_at(program: &str) -> Vec<(usize, &str)> {
    let mut at = 0;
    program
        .split('\n')
        .map(|line| {
            let start = at;
            at += line.len() + 1;
            (start, line)
        })
        .collect()
}

fn placement_heads(program: &str) -> Vec<(usize, usize)> {
    lines_at(program)
        .into_iter()
        .filter(|(_, line)| {
            line.starts_with(' ')
                && line.trim_start().starts_with("for ")
                && !line.contains(" into ")
        })
        .map(|(start, line)| {
            let indent = line.len() - line.trim_start().len();
            (start + indent + "for ".len(), start + line.trim_end().len())
        })
        .collect()
}

fn let_lines<'a>(program: &'a str, head: &str) -> Vec<(usize, &'a str)> {
    let iterable = head.split_once(" in ").map_or("", |(_, iterable)| iterable);
    let names: Vec<&str> = iterable
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| word.starts_with(|c: char| c.is_alphabetic() || c == '_'))
        .collect();
    let lines = lines_at(program);
    let mut found = Vec::new();
    for (at, (start, line)) in lines.iter().enumerate() {
        let joined = match lines.get(at + 1) {
            Some((next, text)) if let_word(line.trim()) => {
                program.get(*start..next + text.len()).unwrap_or(line)
            }
            _ => line,
        };
        let words: Vec<&str> = joined
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .collect();
        if words.iter().any(|word| let_word(word)) && words.iter().any(|word| names.contains(word))
        {
            found.push((*start, joined));
        }
    }
    found
}

fn let_word(word: &str) -> bool {
    word.starts_with("let")
        || word
            .strip_suffix("let")
            .is_some_and(|lead| lead.chars().all(|c| c.is_ascii_digit()))
}

pub fn runs_no_iteration(program: &str, new: &[Drawn]) -> bool {
    let within = |lo: usize, hi: usize| {
        new.iter()
            .any(|drawn| drawn.0 != "no-entry-instance" && drawn.1 >= lo && drawn.2 <= hi)
    };
    placement_heads(program).into_iter().any(|(lo, hi)| {
        within(lo, hi)
            || let_lines(program, program.get(lo..hi).unwrap_or_default())
                .into_iter()
                .any(|(start, line)| within(start, start + line.len() + 1) || unclosed(line))
    })
}

fn iterable_of_a_refused_init(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    if change != Change::Added
        || at.0 != "iterable-type"
        || name.is_empty()
        || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || !seen.after(at).starts_with(" into ")
    {
        return false;
    }
    let head = format!("state {name}(");
    lines_at(seen.program).into_iter().any(|(start, line)| {
        line.starts_with(&head)
            && line
                .rsplit_once("): ")
                .is_some_and(|(_, ty)| !ty.is_empty() && ty.chars().all(char::is_alphanumeric))
            && seen.old.iter().any(|drawn| {
                drawn.0 == "type-mismatch" && drawn.1 >= start && drawn.2 <= start + line.len()
            })
    })
}

fn variable_array_cells(program: &str) -> Vec<&str> {
    program
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.strip_prefix("state ")?.split_once('(')?;
            let (_, ty) = rest.rsplit_once("): ")?;
            (ty.contains("[..") && !ty.contains(' ')).then_some(name)
        })
        .collect()
}

fn variable_array_condition(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Removed
        && at.0 == "condition-type"
        && a_condition(seen, at)
        && variable_array_cells(seen.program).contains(&seen.text(at))
        && seen.new.iter().any(|drawn| {
            drawn.0 == "condition-type"
                && drawn.1 < at.1
                && drawn.2 > at.2
                && seen
                    .program
                    .get(drawn.1..at.1)
                    .is_some_and(|head| head.trim_start_matches('(') == "if ")
        })
}

fn names_a_variable_array_cell(seen: &Seen<'_>, text: &str) -> bool {
    let cells = variable_array_cells(seen.program);
    text.split(|character: char| !character.is_alphanumeric() && character != '_')
        .any(|word| cells.contains(&word))
}

fn operator_over_a_variable_shape_choice(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Removed
        && at.0 == "type-mismatch"
        && seen.new.iter().any(|drawn| {
            drawn.0 == "type-mismatch"
                && drawn.1 > at.1
                && drawn.2 <= at.2
                && seen.text(drawn).starts_with("if ")
                && names_a_variable_array_cell(seen, seen.text(drawn))
        })
}

fn arm_of_a_refused_condition(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let all = choices(seen.program);
    if change == Change::Removed && at.0 == "integer-cell" {
        let name = seen.text(at);
        return cell_reads(seen.program, name).iter().any(|(lo, hi)| {
            all.iter().any(|choice| {
                condition_refused(seen, choice)
                    && all.iter().any(|outer| {
                        outer.condition.0 <= choice.span.0 && choice.span.1 <= outer.condition.1
                    })
                    && choice.arms.iter().any(|arm| arm.0 <= *lo && *hi <= arm.1)
            })
        });
    }
    if change == Change::Added
        && at.0 == "type-mismatch"
        && all
            .iter()
            .any(|choice| condition_refused(seen, choice) && choice.span == (at.1, at.2))
    {
        return true;
    }
    if change == Change::Added && at.0 == "integer-cell" {
        let name = seen.text(at);
        return cell_reads(seen.program, name).iter().any(|(lo, hi)| {
            choices(seen.program).iter().any(|choice| {
                condition_refused(seen, choice)
                    && choice.arms.iter().any(|arm| arm.0 <= *lo && *hi <= arm.1)
            })
        });
    }
    change == Change::Added
        && matches!(
            at.0.as_str(),
            "index-out-of-range" | "does-not-fit" | "type-mismatch"
        )
        && choices(seen.program).iter().any(|choice| {
            condition_refused(seen, choice)
                && choice.arms.iter().any(|arm| arm.0 <= at.1 && at.2 <= arm.1)
        })
}

fn refused_beside_a_variable_array(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Added
        && matches!(
            at.0.as_str(),
            "index-out-of-range" | "array-operand" | "type-mismatch" | "index-type"
        )
        && !variable_array_cells(seen.program).is_empty()
        && seen.new.iter().any(|drawn| {
            drawn != at
                && matches!(
                    drawn.0.as_str(),
                    "condition-type" | "array-operand" | "operand-type" | "index-type"
                )
                && ((drawn.1 <= at.1 && at.2 <= drawn.2)
                    || (at.1 <= drawn.1 && drawn.2 <= at.2)
                    || (drawn.2 <= at.1
                        && seen
                            .program
                            .get(drawn.2..at.1)
                            .is_some_and(|between| between.contains(" then "))))
        })
}

fn variable_array_index(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let cells = variable_array_cells(seen.program);
    let text = seen.text(at);
    let indexed_by_cell = |text: &str| {
        cells.iter().any(|cell| {
            text.ends_with(&format!("[{cell}]")) || text.ends_with(&format!("[{cell}..]"))
        })
    };
    match change {
        Change::Removed => {
            at.0 == "not-an-array" && holds(seen.new, "index-type", at) && indexed_by_cell(text)
        }
        Change::Added => {
            at.0 == "index-type"
                && indexed_by_cell(text)
                && seen
                    .new
                    .iter()
                    .any(|drawn| drawn.0 == "not-an-array" && drawn.1 == at.1 && drawn.2 < at.2)
        }
        Change::Restated => false,
    }
}

fn untaken_arms(program: &str) -> Vec<(usize, usize)> {
    let mut arms = Vec::new();
    for (head, taken) in [("if true then ", true), ("if false then ", false)] {
        for (start, _) in program.match_indices(head) {
            if !word_start(program, start) {
                continue;
            }
            let then_start = start + head.len();
            let mut depth = 0i32;
            let mut else_at = None;
            for (offset, character) in program[then_start..].char_indices() {
                match character {
                    '(' | '[' => depth += 1,
                    ')' | ']' => depth -= 1,
                    '\n' if depth <= 0 => break,
                    _ => {}
                }
                if depth < 0 {
                    break;
                }
                if depth == 0 && program[then_start + offset..].starts_with(" else ") {
                    else_at = Some(then_start + offset);
                    break;
                }
            }
            let Some(else_at) = else_at else {
                continue;
            };
            if !taken {
                arms.push((then_start, else_at));
                continue;
            }
            let else_start = else_at + " else ".len();
            let mut depth = 0i32;
            let mut end = program.len();
            for (offset, character) in program[else_start..].char_indices() {
                match character {
                    '(' | '[' => depth += 1,
                    ')' | ']' => depth -= 1,
                    '\n' if depth <= 0 => {
                        end = else_start + offset;
                        break;
                    }
                    _ => {}
                }
                if depth < 0 {
                    end = else_start + offset;
                    break;
                }
            }
            arms.push((else_start, end));
        }
    }
    arms
}

fn line_span(program: &str, at: usize) -> (usize, usize) {
    let start = program
        .get(..at)
        .and_then(|text| text.rfind('\n'))
        .map_or(0, |at| at + 1);
    let end = program
        .get(at..)
        .and_then(|text| text.find('\n'))
        .map_or(program.len(), |offset| at + offset);
    (start, end)
}

fn only_untaken(program: &str, name: &str, arms: &[(usize, usize)], depth: usize) -> bool {
    if depth > 4 {
        return false;
    }
    let reads: Vec<(usize, usize)> = cell_reads(program, name)
        .into_iter()
        .filter(|(lo, _)| {
            let before = program.get(..*lo).unwrap_or_default();
            !before.ends_with("fn ") && !before.ends_with("let ")
        })
        .collect();
    !reads.is_empty()
        && reads.iter().all(|(lo, hi)| {
            if arms.iter().any(|(start, end)| start <= lo && hi <= end) {
                return true;
            }
            let (start, end) = line_span(program, *lo);
            let line = program.get(start..end).unwrap_or_default();
            let trimmed = line.trim_start();
            let holder = if let Some(rest) = trimmed.strip_prefix("fn ") {
                rest.split_once('(').map(|(function, _)| function)
            } else if let Some(rest) = trimmed.strip_prefix("let ") {
                rest.split_once(" = ").map(|(local, _)| local)
            } else {
                None
            };
            holder.is_some_and(|holder| only_untaken(program, holder, arms, depth + 1))
        })
}

fn integer_cell_read_only_untaken(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    change == Change::Removed
        && at.0 == "integer-cell"
        && !name.is_empty()
        && only_untaken(seen.program, name, &untaken_arms(seen.program), 0)
}

fn integer_cell_through_a_local_let(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let name = seen.text(at);
    change == Change::Added
        && at.0 == "integer-cell"
        && !name.is_empty()
        && seen.program.lines().any(|line| {
            line.starts_with("    ")
                && line
                    .trim_start()
                    .strip_prefix("let ")
                    .and_then(|rest| rest.split_once(" = "))
                    .is_some_and(|(_, value)| value == name)
        })
}

fn condition_cut_by_a_line_fault(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    change == Change::Removed
        && at.0 == "condition-type"
        && seen.text(at).starts_with("if ")
        && seen
            .old
            .iter()
            .chain(seen.new.iter())
            .any(|drawn| line_fault(&drawn.0) && drawn.1 == at.2)
}

fn open_history_join(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change == Change::Added && at.0 == "type-mismatch" {
        let name = seen.text(at);
        return !name.is_empty()
            && name
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_')
            && seen.old.iter().any(|drawn| {
                drawn.0 == "array-operand"
                    && open_history_join(seen, Change::Removed, drawn)
                    && lines_at(seen.program)
                        .get(seen.line(drawn.1))
                        .is_some_and(|(_, line)| line.starts_with(&format!("state {name}(")))
            });
    }
    if change != Change::Removed || at.0 != "array-operand" {
        return false;
    }
    let text = seen.text(at);
    let open = |name: &str| {
        seen.program
            .lines()
            .any(|line| line.starts_with(&format!("state {name} = ")))
    };
    text.match_indices(".past(").any(|(dot, _)| {
        let name: String = text[..dot]
            .chars()
            .rev()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect();
        let call = &text[dot + 6..];
        let one = call
            .split_once(')')
            .is_some_and(|(argument, _)| !argument.contains(','));
        !name.is_empty() && one && open(&name)
    })
}

fn whole_let_line<'a>(seen: &Seen<'a>, at: &Drawn) -> Option<(&'a str, &'a str)> {
    let (start, line) = lines_at(seen.program)
        .into_iter()
        .find(|(start, _)| *start == at.1)?;
    if at.2 != start + line.len() + 1 && at.2 != start + line.len() {
        return None;
    }
    let (head, _) = line.strip_prefix("let ")?.split_once(" = ")?;
    let (name, ty) = head.split_once(": ")?;
    ty.ends_with(']').then_some((name, ty))
}

fn variable_let_element(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change != Change::Removed || !matches!(at.0.as_str(), "type-mismatch" | "does-not-fit") {
        return false;
    }
    let Some((_, ty)) = whole_let_line(seen, at) else {
        return false;
    };
    ty.contains("[..")
        && seen
            .new
            .iter()
            .any(|drawn| drawn.0 == at.0 && drawn.1 > at.1 && drawn.2 <= at.2)
}

fn refused_array_let_keeps_its_type(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change != Change::Added || !matches!(at.0.as_str(), "type-mismatch" | "operand-type") {
        return false;
    }
    seen.old.iter().any(|drawn| {
        matches!(drawn.0.as_str(), "capacity-exceeded" | "type-mismatch")
            && seen.line(drawn.1) != seen.line(at.1)
            && whole_let_line(seen, drawn).is_some_and(|(name, _)| reads_word(seen.text(at), name))
    })
}

fn unrun_cycle(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let unrun = |set: &[Drawn]| set.iter().any(|drawn| drawn.0 == "unrun-cell");
    if change == Change::Added && at.0 == "unrun-cell" {
        let name = seen.text(at);
        return !unrun(seen.old)
            && seen
                .old
                .iter()
                .any(|drawn| unrun_cycle(seen, Change::Removed, drawn))
            && lines_at(seen.program)
                .get(seen.line(at.1))
                .is_some_and(|(start, line)| {
                    *start + "state ".len() == at.1 && line.starts_with(&format!("state {name}"))
                });
    }
    if change != Change::Removed || at.0 != "not-an-array" || !unrun(seen.new) {
        return false;
    }
    let text = seen.text(at);
    text.contains(".past(")
        && !text.contains(", ")
        && seen
            .program
            .get(at.2..)
            .is_some_and(|after| after.starts_with(")[") || after.starts_with('['))
}

fn unclosed(line: &str) -> bool {
    let opened = line
        .chars()
        .filter(|c| matches!(c, '(' | '[' | '{'))
        .count();
    let closed = line
        .chars()
        .filter(|c| matches!(c, ')' | ']' | '}'))
        .count();
    opened > closed
}

fn gate_beside_its_faults(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let line = seen.before(at).rsplit('\n').next().unwrap_or_default();
    let Some(gate) = line.find(" when ") else {
        return false;
    };
    let start = at.1 - (line.len() - gate);
    if change == Change::Added && at.0 == "does-not-fit" {
        return at.1 > start
            && seen.new.iter().any(|drawn| {
                drawn.0 == "condition-type" && drawn.1 > start && drawn.1 <= at.1 && at.2 <= drawn.2
            });
    }
    if change == Change::Added && at.0 == "constant-overflow" {
        let text = seen.text(at);
        return at.1 > start
            && text.starts_with("-(if ")
            && seen.program.lines().any(|declared| {
                declared
                    .strip_prefix("state ")
                    .and_then(|rest| rest.split_once('('))
                    .is_some_and(|(name, rest)| rest.ends_with("): u8") && reads_word(text, name))
            });
    }
    if change == Change::Removed {
        let text = seen.text(at);
        return at.0 == "condition-type"
            && at.1 > start
            && (seen.new.iter().any(|drawn| drawn == at)
                || (text.ends_with("..]")
                    && seen.old.iter().any(|drawn| {
                        drawn.0 == "index-out-of-range" && drawn.1 > at.1 && drawn.2 <= at.2
                    })
                    && seen.new.iter().any(|drawn| {
                        drawn.0 == "condition-type" && drawn.1 < at.1 && at.2 < drawn.2
                    })));
    }
    change == Change::Added
        && at.1 > start
        && matches!(
            at.0.as_str(),
            "condition-type" | "type-mismatch" | "array-operand"
        )
        && !line.trim_start().starts_with("when")
        && seen.old.iter().any(|drawn| {
            drawn.0 != at.0 && drawn.1 > start && seen.line(drawn.1) == seen.line(at.1)
        })
}

fn fixed_type_operand_of_a_cell_rule(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change == Change::Restated || at.0 != "does-not-fit" {
        return false;
    }
    let literal = seen.text(at);
    if literal.is_empty() || !literal.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let line = seen.before(at).rsplit('\n').next().unwrap_or_default();
    let cell = |name: &str| seen.program.contains(&format!("state {name}("));
    let converted = |text: &str| {
        ["u8(", "i32("].iter().any(|conversion| {
            text.rsplit_once(conversion).is_some_and(|(_, rest)| {
                rest.split_once(')').is_some_and(|(name, operator)| {
                    cell(name) && matches!(operator, " + " | " - " | " * ")
                })
            })
        })
    };
    if change == Change::Removed {
        return line.contains("/(") && converted(line);
    }
    if (line.starts_with("state ") || line.contains("/(")) && converted(line) {
        return true;
    }
    let Some(name) = line
        .trim_start()
        .strip_prefix("Note when ")
        .and_then(|rest| rest.strip_suffix(" > "))
    else {
        return false;
    };
    ["u8(", "i32("].iter().any(|conversion| {
        seen.program
            .contains(&format!("state {name} = {conversion}"))
    })
}

fn range_bound_beside_a_faulty_fold(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change != Change::Added || at.0 != "range-bound" {
        return false;
    }
    let line = seen.line(at.1);
    let Some((_, text)) = lines_at(seen.program).get(line).copied() else {
        return false;
    };
    let rule = text.starts_with("state ") && text.contains("into");
    let open = text.trim_end().ends_with(':');
    rule && !seen
        .old
        .iter()
        .any(|drawn| drawn.0 == "range-bound" && drawn.1 == at.1)
        && seen.old.iter().any(|drawn| {
            let at_line = seen.line(drawn.1);
            drawn.0 != "range-bound"
                && drawn.1 >= at.2
                && (at_line == line || (open && at_line == line + 1))
        })
}

fn read_of_a_cell_beside_its_refused_init(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change != Change::Added || !matches!(at.0.as_str(), "unknown-attribute" | "array-operand") {
        return false;
    }
    let words: Vec<&str> = seen
        .text(at)
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
        .collect();
    lines_at(seen.program).into_iter().any(|(start, line)| {
        let Some((cell, rest)) = line
            .strip_prefix("state ")
            .and_then(|rest| rest.split_once('('))
        else {
            return false;
        };
        words.contains(&cell)
            && rest
                .rsplit_once("): ")
                .is_some_and(|(_, ty)| !ty.is_empty() && ty.chars().all(char::is_alphanumeric))
            && seen.old.iter().any(|drawn| {
                drawn.0 == "type-mismatch" && drawn.1 >= start && drawn.2 <= start + line.len()
            })
    })
}

fn slice_with_a_faulty_bound(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    let text = seen.text(at);
    change == Change::Added
        && at.0 == "type-mismatch"
        && text.contains("..")
        && text.ends_with(']')
        && seen.old.iter().any(|drawn| {
            matches!(drawn.0.as_str(), "expression-syntax" | "misplaced-caret")
                && drawn.1 >= at.1
                && drawn.2 <= at.2
        })
}

fn declared_name(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("let ")
        .or_else(|| line.strip_prefix("fn "))?;
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    rest.get(..end)
}

fn item_words(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
}

fn member_call_in_an_unused_value(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change != Change::Removed || !matches!(at.0.as_str(), "not-callable" | "unknown-name") {
        return false;
    }
    let member = seen.text(at).rsplit('.').next().unwrap_or_default();
    if !["size", "capacity", "bytes", "keys", "values"].contains(&member)
        || !seen.after(at).starts_with('(')
    {
        return false;
    }
    let lines = lines_at(seen.program);
    let mut items: Vec<(Option<&str>, String)> = Vec::new();
    for (_, line) in &lines {
        match items.last_mut() {
            Some((_, text)) if line.starts_with(' ') => {
                text.push('\n');
                text.push_str(line);
            }
            _ => items.push((declared_name(line), (*line).to_string())),
        }
    }
    let mut used: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut grown = true;
    while grown {
        grown = false;
        for (name, text) in &items {
            if name.is_some_and(|name| !used.contains(name)) {
                continue;
            }
            let own = name.unwrap_or_default();
            for word in item_words(text) {
                if word != own && used.insert(word) {
                    grown = true;
                }
            }
        }
    }
    let line = seen.line(at.1);
    let Some((start, _)) = lines.get(line) else {
        return false;
    };
    let mut owner = None;
    for (begin, text) in &lines {
        if *begin > *start {
            break;
        }
        if !text.starts_with(' ') {
            owner = declared_name(text);
        }
    }
    owner.is_some_and(|name| !used.contains(name))
}

fn member_name_in_an_unexpanded_loop_body(seen: &Seen<'_>, change: Change, at: &Drawn) -> bool {
    if change != Change::Added
        || !matches!(
            at.0.as_str(),
            "unknown-attribute" | "unknown-flag" | "unknown-parameter"
        )
    {
        return false;
    }
    let lines = lines_at(seen.program);
    let line = seen.line(at.1);
    let Some((_, text)) = lines.get(line) else {
        return false;
    };
    let depth = text.len() - text.trim_start().len();
    let head = lines.iter().take(line).rev().find(|(_, earlier)| {
        let indent = earlier.len() - earlier.trim_start().len();
        indent < depth && !earlier.trim().is_empty()
    });
    let Some((start, head)) = head else {
        return false;
    };
    head.trim_start().starts_with("for ")
        && seen
            .old
            .iter()
            .any(|drawn| drawn.1 >= *start && drawn.2 <= start + head.len())
}
