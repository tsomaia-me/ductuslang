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
        "Clip c:\n    Note/(([1, 2] is 300000000000)[i32(true) div q])\n",
        &[
            ("unknown-name", "q"),
            ("array-operand", "[1, 2] is 300000000000"),
            ("not-an-array", "[1, 2] is 300000000000"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(([1, 2] is 300000000000)[i32(true) div q])\n",
        &[
            ("unknown-name", "q"),
            ("array-operand", "[1, 2] is 300000000000"),
            ("not-an-array", "[1, 2] is 300000000000"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((not q)[[1, 2][5] div q])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("not-an-array", "not q"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((not q)[[1, 2][5] div q])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("not-an-array", "not q"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if q then i32(true) else true)[[1, 2][1.5..]])\n",
        &[
            ("unknown-name", "q"),
            (
                "index-type",
                "(if q then i32(true) else true)[[1, 2][1.5..]]",
            ),
            ("index-type", "1.5"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if q then i32(true) else true)[[1, 2][1.5..]])\n",
        &[
            ("unknown-name", "q"),
            (
                "index-type",
                "(if q then i32(true) else true)[[1, 2][1.5..]]",
            ),
            ("index-type", "1.5"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((i32(true) and [1, 2][5])[i32(true) / q..])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "i32(true) and [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("operand-type", "i32(true) and [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((i32(true) and [1, 2][5])[i32(true) / q..])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "i32(true) and [1, 2][5]"),
            ("index-out-of-range", "5"),
            ("operand-type", "i32(true) and [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note/((true is q)[if true then [1, 2][5] else [1, 2]])\n",
        &[
            ("unknown-name", "q"),
            ("type-mismatch", "true is q"),
            ("type-mismatch", "if true then [1, 2][5] else [1, 2]"),
            ("not-an-array", "true is q"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note | velocity=((true is q)[if true then [1, 2][5] else [1, 2]])\n",
        &[
            ("unknown-name", "q"),
            ("type-mismatch", "true is q"),
            ("type-mismatch", "if true then [1, 2][5] else [1, 2]"),
            ("not-an-array", "true is q"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((q < i32(true))[i32(true) div q])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("not-an-array", "q < i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((q < i32(true))[i32(true) div q])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("not-an-array", "q < i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note/((true < q)[i32(true)[[1, 2]]])\n",
        &[
            ("unknown-name", "q"),
            ("type-mismatch", "true < q"),
            ("index-type", "i32(true)[[1, 2]]"),
            ("not-an-array", "true < q"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note | velocity=((true < q)[i32(true)[[1, 2]]])\n",
        &[
            ("unknown-name", "q"),
            ("type-mismatch", "true < q"),
            ("index-type", "i32(true)[[1, 2]]"),
            ("not-an-array", "true < q"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((300000000000 < q)[if i32(true) then q else [1, 2]])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("not-an-array", "300000000000 < q"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((300000000000 < q)[if i32(true) then q else [1, 2]])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("not-an-array", "300000000000 < q"),
            ("conversion-type", "i32(true)"),
            ("condition-type", "i32(true)"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note/((1.5 is q)[i32(true)[1]])\n",
        &[
            ("unknown-name", "q"),
            ("type-mismatch", "1.5 is q"),
            ("not-an-array", "1.5 is q"),
            ("not-an-array", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note | velocity=((1.5 is q)[i32(true)[1]])\n",
        &[
            ("unknown-name", "q"),
            ("type-mismatch", "1.5 is q"),
            ("not-an-array", "1.5 is q"),
            ("not-an-array", "i32(true)"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(([1, 2][5] - 1.5)[1 * q])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "[1, 2][5] - 1.5"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(([1, 2][5] - 1.5)[1 * q])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "[1, 2][5] - 1.5"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((1.5 / 1.5)[if true then q else [1, 2]..])\n",
        &[("unknown-name", "q"), ("not-an-array", "1.5 / 1.5")],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((1.5 / 1.5)[if true then q else [1, 2]..])\n",
        &[("unknown-name", "q"), ("not-an-array", "1.5 / 1.5")],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note/(([1, 2][5] or q)[true[true]..])\n",
        &[
            ("unknown-name", "q"),
            ("type-mismatch", "[1, 2][5] or q"),
            ("not-an-array", "[1, 2][5] or q"),
            ("not-an-array", "true"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note | velocity=(([1, 2][5] or q)[true[true]..])\n",
        &[
            ("unknown-name", "q"),
            ("type-mismatch", "[1, 2][5] or q"),
            ("not-an-array", "[1, 2][5] or q"),
            ("not-an-array", "true"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if true then q else i32(true))[[1, 2][5] / 300000000000])\n",
        &[
            ("unknown-name", "q"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if true then q else i32(true))[[1, 2][5] / 300000000000])\n",
        &[
            ("unknown-name", "q"),
            ("index-out-of-range", "5"),
            ("conversion-type", "i32(true)"),
            ("does-not-fit", "300000000000"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((q < [1, 2])[true * q..])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("array-operand", "q < [1, 2]"),
            ("not-an-array", "q < [1, 2]"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((q < [1, 2])[true * q..])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("array-operand", "q < [1, 2]"),
            ("not-an-array", "q < [1, 2]"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(([1, 2][5] < true)[i32(true) + q..])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "[1, 2][5] < true"),
            ("index-out-of-range", "5"),
            ("operand-type", "[1, 2][5] < true"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(([1, 2][5] < true)[i32(true) + q..])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "[1, 2][5] < true"),
            ("index-out-of-range", "5"),
            ("operand-type", "[1, 2][5] < true"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(([1, 2][5] < 300000000000)[true / q])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "[1, 2][5] < 300000000000"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(([1, 2][5] < 300000000000)[true / q])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "[1, 2][5] < 300000000000"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((q and [1, 2])[q - i32(true)])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("array-operand", "q and [1, 2]"),
            ("not-an-array", "q and [1, 2]"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((q and [1, 2])[q - i32(true)])\n",
        &[
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("array-operand", "q and [1, 2]"),
            ("not-an-array", "q and [1, 2]"),
            ("conversion-type", "i32(true)"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((1.5 < [1, 2][5])[1 % q])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "1.5 < [1, 2][5]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((1.5 < [1, 2][5])[1 % q])\n",
        &[
            ("unknown-name", "q"),
            ("not-an-array", "1.5 < [1, 2][5]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if 1 then q else [1, 2])\n",
        &[("condition-type", "1"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if 1 then q else [1, 2])\n",
        &[("condition-type", "1"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note/(if 1 then [1, 2] else q)\n",
        &[("condition-type", "1"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if 1 then [1, 2] else q)\n",
        &[("condition-type", "1"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note/(if 1.5 then q else [1, 2])\n",
        &[("condition-type", "1.5"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if 1.5 then q else [1, 2])\n",
        &[("condition-type", "1.5"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note/(if 1.5 then [1, 2] else q)\n",
        &[("condition-type", "1.5"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if 1.5 then [1, 2] else q)\n",
        &[("condition-type", "1.5"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note/(if [1, 2][5] then q else [1, 2])\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if [1, 2][5] then q else [1, 2])\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if [1, 2][5] then [1, 2] else q)\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if [1, 2][5] then [1, 2] else q)\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if 300000000000 then q else [1, 2])\n",
        &[("condition-type", "300000000000"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if 300000000000 then q else [1, 2])\n",
        &[("condition-type", "300000000000"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note/(if 300000000000 then [1, 2] else q)\n",
        &[("condition-type", "300000000000"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if 300000000000 then [1, 2] else q)\n",
        &[("condition-type", "300000000000"), ("unknown-name", "q")],
    ),
    (
        "476",
        "Clip c:\n    Note/(if i32(true) then q else [1, 2])\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if i32(true) then q else [1, 2])\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if i32(true) then [1, 2] else q)\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if i32(true) then [1, 2] else q)\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if [1, 2][5] then [1, 2] else true)[i32(true) and q])\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-type", "i32(true) and q"),
            ("type-mismatch", "if [1, 2][5] then [1, 2] else true"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if [1, 2][5] then [1, 2] else true)[i32(true) and q])\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-type", "i32(true) and q"),
            ("type-mismatch", "if [1, 2][5] then [1, 2] else true"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note/(if (1 div i32(true)) then (if [1, 2] then q else q) else (q or [1, 2]))\n",
        &[
            ("array-operand", "q or [1, 2]"),
            ("condition-type", "1 div i32(true)"),
            ("condition-type", "if [1, 2] then q else q"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "q or [1, 2]"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476, the twin pass's (R4b)",
        "Clip c:\n    Note | velocity=(if (1 div i32(true)) then (if [1, 2] then q else q) else (q or [1, 2]))\n",
        &[
            ("array-operand", "q or [1, 2]"),
            ("condition-type", "1 div i32(true)"),
            ("condition-type", "if [1, 2] then q else q"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "q or [1, 2]"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((q div 300000000000)[if i32(true) then true else [1, 2]..])\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((q div 300000000000)[if i32(true) then true else [1, 2]..])\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if i32(true) then true else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if 1.5 then [1, 2] else q)[1 and [1, 2]..])\n",
        &[
            ("array-operand", "1 and [1, 2]"),
            ("condition-type", "1.5"),
            ("index-type", "1 and [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if 1.5 then [1, 2] else q)[1 and [1, 2]..])\n",
        &[
            ("array-operand", "1 and [1, 2]"),
            ("condition-type", "1.5"),
            ("index-type", "1 and [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (if true then [1, 2][5] else 300000000000) then (if q then 300000000000 else 1.5) else (if 300000000000 then 1 else q))\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "if true then [1, 2][5] else 300000000000"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (if true then [1, 2][5] else 300000000000) then (if q then 300000000000 else 1.5) else (if 300000000000 then 1 else q))\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "if true then [1, 2][5] else 300000000000"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if i32(true) then 1.5 else q) is (if 1 then [1, 2] else 300000000000))\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1 then [1, 2] else 300000000000"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if i32(true) then 1.5 else q) is (if 1 then [1, 2] else 300000000000))\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1 then [1, 2] else 300000000000"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if i32(true) then [1, 2] else i32(true))[if [1, 2] then q else 1])\n",
        &[
            ("condition-type", "i32(true)"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if i32(true) then [1, 2] else i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if i32(true) then [1, 2] else i32(true))[if [1, 2] then q else 1])\n",
        &[
            ("condition-type", "i32(true)"),
            ("condition-type", "if [1, 2] then q else 1"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if i32(true) then [1, 2] else i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(([1, 2] + q)[if 1.5 then [1, 2] else i32(true)])\n",
        &[
            ("condition-type", "1.5"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1.5 then [1, 2] else i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(([1, 2] + q)[if 1.5 then [1, 2] else i32(true)])\n",
        &[
            ("condition-type", "1.5"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1.5 then [1, 2] else i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if ([1, 2][5] div 1) then (q and [1, 2]) else (if 1.5 then 1.5 else [1, 2][5]))\n",
        &[
            ("array-operand", "q and [1, 2]"),
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5] div 1"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if ([1, 2][5] div 1) then (q and [1, 2]) else (if 1.5 then 1.5 else [1, 2][5]))\n",
        &[
            ("array-operand", "q and [1, 2]"),
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5] div 1"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(i32(if i32(true) then q else [1, 2]))\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(i32(if i32(true) then q else [1, 2]))\n",
        &[
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (1 + 1.5) then (true < true) else (if i32(true) then q else [1, 2]))\n",
        &[
            ("condition-type", "1 + 1.5"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "true < true"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (1 + 1.5) then (true < true) else (if i32(true) then q else [1, 2]))\n",
        &[
            ("condition-type", "1 + 1.5"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "true < true"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if 1 then true else [1, 2])[i32(true) div q])\n",
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1 then true else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if 1 then true else [1, 2])[i32(true) div q])\n",
        &[
            ("condition-type", "1"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1 then true else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if 1 then [1, 2][5] else [1, 2])[q is q..])\n",
        &[
            ("condition-type", "1"),
            ("index-type", "q is q"),
            ("type-mismatch", "if 1 then [1, 2][5] else [1, 2]"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if 1 then [1, 2][5] else [1, 2])[q is q..])\n",
        &[
            ("condition-type", "1"),
            ("index-type", "q is q"),
            ("type-mismatch", "if 1 then [1, 2][5] else [1, 2]"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (if 1.5 then 300000000000 else q) then (true % 1) else (if 1.5 then [1, 2][5] else [1, 2]))\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "1.5"),
            ("operand-type", "true % 1"),
            ("type-mismatch", "if 1.5 then [1, 2][5] else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (if 1.5 then 300000000000 else q) then (true % 1) else (if 1.5 then [1, 2][5] else [1, 2]))\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "1.5"),
            ("operand-type", "true % 1"),
            ("type-mismatch", "if 1.5 then [1, 2][5] else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (q and [1, 2][5]) then (if [1, 2][5] then q else [1, 2]) else (if [1, 2][5] then [1, 2][5] else i32(true)))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (q and [1, 2][5]) then (if [1, 2][5] then q else [1, 2]) else (if [1, 2][5] then [1, 2][5] else i32(true)))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(([1, 2] % 300000000000)[if 1.5 then q else [1, 2]..])\n",
        &[
            ("array-operand", "[1, 2] % 300000000000"),
            ("condition-type", "1.5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(([1, 2] % 300000000000)[if 1.5 then q else [1, 2]..])\n",
        &[
            ("array-operand", "[1, 2] % 300000000000"),
            ("condition-type", "1.5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((i32(true)[[1, 2][5]])[if [1, 2][5] then [1, 2] else q..])\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("not-an-array", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((i32(true)[[1, 2][5]])[if [1, 2][5] then [1, 2] else q..])\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("not-an-array", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if 300000000000 then q else [1, 2]) is (if 1 then i32(true) else [1, 2]))\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1 then i32(true) else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if 300000000000 then q else [1, 2]) is (if 1 then i32(true) else [1, 2]))\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "300000000000"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1 then i32(true) else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if ([1, 2][5] - i32(true)) then (if i32(true) then 1.5 else 1.5) else (q / [1, 2][5]))\n",
        &[
            ("condition-type", "[1, 2][5] - i32(true)"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if ([1, 2][5] - i32(true)) then (if i32(true) then 1.5 else 1.5) else (q / [1, 2][5]))\n",
        &[
            ("condition-type", "[1, 2][5] - i32(true)"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (if 1.5 then i32(true) else true) then (if [1, 2][5] then [1, 2] else 1.5) else (if [1, 2] then 1.5 else q))\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1.5 else q"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2][5] then [1, 2] else 1.5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (if 1.5 then i32(true) else true) then (if [1, 2][5] then [1, 2] else 1.5) else (if [1, 2] then 1.5 else q))\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5]"),
            ("condition-type", "if [1, 2] then 1.5 else q"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if [1, 2][5] then [1, 2] else 1.5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if 300000000000 then true else [1, 2]) and (if [1, 2] then [1, 2] else q))\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "if [1, 2] then [1, 2] else q"),
            ("type-mismatch", "if 300000000000 then true else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if 300000000000 then true else [1, 2]) and (if [1, 2] then [1, 2] else q))\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "if [1, 2] then [1, 2] else q"),
            ("type-mismatch", "if 300000000000 then true else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if 1.5 then true else [1, 2])[[1, 2] - q])\n",
        &[
            ("array-operand", "[1, 2] - q"),
            ("condition-type", "1.5"),
            ("type-mismatch", "if 1.5 then true else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if 1.5 then true else [1, 2])[[1, 2] - q])\n",
        &[
            ("array-operand", "[1, 2] - q"),
            ("condition-type", "1.5"),
            ("type-mismatch", "if 1.5 then true else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (if 1 then [1, 2] else 300000000000) then (if q then true else [1, 2][5]) else (if 1 then true else 1))\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "1"),
            ("type-mismatch", "if 1 then [1, 2] else 300000000000"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (if 1 then [1, 2] else 300000000000) then (if q then true else [1, 2][5]) else (if 1 then true else 1))\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "1"),
            ("type-mismatch", "if 1 then [1, 2] else 300000000000"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (u8(i32(true))) then (if 300000000000 then 1.5 else q) else (i32(true) / 300000000000))\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "u8(i32(true))"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (u8(i32(true))) then (if 300000000000 then 1.5 else q) else (i32(true) / 300000000000))\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "u8(i32(true))"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (i32(true) * 1) then (q % [1, 2]) else (if [1, 2] then [1, 2] else 300000000000))\n",
        &[
            ("array-operand", "q % [1, 2]"),
            ("condition-type", "i32(true) * 1"),
            ("condition-type", "if [1, 2] then [1, 2] else 300000000000"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (i32(true) * 1) then (q % [1, 2]) else (if [1, 2] then [1, 2] else 300000000000))\n",
        &[
            ("array-operand", "q % [1, 2]"),
            ("condition-type", "i32(true) * 1"),
            ("condition-type", "if [1, 2] then [1, 2] else 300000000000"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (if q then i32(true) else q) then (if [1, 2] then 1 else 300000000000) else (if 1 then 300000000000 else [1, 2]))\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if [1, 2] then 1 else 300000000000"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1 then 300000000000 else [1, 2]"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (if q then i32(true) else q) then (if [1, 2] then 1 else 300000000000) else (if 1 then 300000000000 else [1, 2]))\n",
        &[
            ("condition-type", "1"),
            ("condition-type", "if [1, 2] then 1 else 300000000000"),
            ("conversion-type", "i32(true)"),
            ("type-mismatch", "if 1 then 300000000000 else [1, 2]"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if ([1, 2][5] div [1, 2][5]) then (q * 1) else (if 1.5 then i32(true) else true))\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5] div [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if ([1, 2][5] div [1, 2][5]) then (q * 1) else (if 1.5 then i32(true) else true))\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "[1, 2][5] div [1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("index-out-of-range", "5"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if [1, 2][5] then 300000000000 else [1, 2])[1 div q..])\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            (
                "type-mismatch",
                "if [1, 2][5] then 300000000000 else [1, 2]",
            ),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if [1, 2][5] then 300000000000 else [1, 2])[1 div q..])\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("index-out-of-range", "5"),
            (
                "type-mismatch",
                "if [1, 2][5] then 300000000000 else [1, 2]",
            ),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (if 1.5 then [1, 2] else i32(true)) then (true + 300000000000) else (if q then true else 300000000000))\n",
        &[
            ("condition-type", "1.5"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "true + 300000000000"),
            ("type-mismatch", "if 1.5 then [1, 2] else i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (if 1.5 then [1, 2] else i32(true)) then (true + 300000000000) else (if q then true else 300000000000))\n",
        &[
            ("condition-type", "1.5"),
            ("conversion-type", "i32(true)"),
            ("operand-type", "true + 300000000000"),
            ("type-mismatch", "if 1.5 then [1, 2] else i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if q then 1.5 else true) is (if [1, 2][5] then [1, 2] else i32(true)))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if [1, 2][5] then [1, 2] else i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if q then 1.5 else true) is (if [1, 2][5] then [1, 2] else i32(true)))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            ("type-mismatch", "if [1, 2][5] then [1, 2] else i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if 1.5 then 300000000000 else [1, 2])[q / 1.5..])\n",
        &[
            ("condition-type", "1.5"),
            ("type-mismatch", "if 1.5 then 300000000000 else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if 1.5 then 300000000000 else [1, 2])[q / 1.5..])\n",
        &[
            ("condition-type", "1.5"),
            ("type-mismatch", "if 1.5 then 300000000000 else [1, 2]"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (if 300000000000 then [1, 2] else 1) then (if 1.5 then [1, 2][5] else [1, 2][5]) else (if q then [1, 2][5] else true))\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "300000000000"),
            ("type-mismatch", "if 300000000000 then [1, 2] else 1"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (if 300000000000 then [1, 2] else 1) then (if 1.5 then [1, 2][5] else [1, 2][5]) else (if q then [1, 2][5] else true))\n",
        &[
            ("condition-type", "1.5"),
            ("condition-type", "300000000000"),
            ("type-mismatch", "if 300000000000 then [1, 2] else 1"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if [1, 2][5] then [1, 2] else 300000000000) < (q % i32(true)))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            (
                "type-mismatch",
                "if [1, 2][5] then [1, 2] else 300000000000",
            ),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if [1, 2][5] then [1, 2] else 300000000000) < (q % i32(true)))\n",
        &[
            ("condition-type", "[1, 2][5]"),
            ("conversion-type", "i32(true)"),
            ("index-out-of-range", "5"),
            (
                "type-mismatch",
                "if [1, 2][5] then [1, 2] else 300000000000",
            ),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (if true then 1.5 else 300000000000) then (i32(true) < 1.5) else (q * q))\n",
        &[
            ("condition-type", "if true then 1.5 else 300000000000"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (if true then 1.5 else 300000000000) then (i32(true) < 1.5) else (q * q))\n",
        &[
            ("condition-type", "if true then 1.5 else 300000000000"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/((if i32(true) then [1, 2] else q)[if i32(true) then 1.5 else i32(true)..])\n",
        &[
            ("condition-type", "i32(true)"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=((if i32(true) then [1, 2] else q)[if i32(true) then 1.5 else i32(true)..])\n",
        &[
            ("condition-type", "i32(true)"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note/(if (q * true) then (if 300000000000 then [1, 2] else 300000000000) else (if i32(true) then q else true))\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            (
                "type-mismatch",
                "if 300000000000 then [1, 2] else 300000000000",
            ),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "Clip c:\n    Note | velocity=(if (q * true) then (if 300000000000 then [1, 2] else 300000000000) else (if i32(true) then q else true))\n",
        &[
            ("condition-type", "300000000000"),
            ("condition-type", "i32(true)"),
            ("conversion-type", "i32(true)"),
            (
                "type-mismatch",
                "if 300000000000 then [1, 2] else 300000000000",
            ),
            ("unknown-name", "q"),
            ("unknown-name", "q"),
        ],
    ),
    (
        "476",
        "state i(1)\nlet xs = [10, 20, 30, 40]\nlet s = xs[i..^1] + xs[..1]Clip c:\n    Note/(s[0] + s.size)\n",
        &[("let-syntax", "Clip")],
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
];

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
