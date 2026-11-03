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
        "476",
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
        "476",
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
        "476",
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
        "476",
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
        ],
    ),
    (
        "476",
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
        "476",
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
        "476",
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
        "476",
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
