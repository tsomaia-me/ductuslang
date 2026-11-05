use crate::compiler::Compiler;
use crate::diagnostics::{Code, Diagnostic};
use crate::syntax::front_end_goldens::source;

struct Case {
    items: &'static [&'static str],
    drawn: &'static [(Code, &'static str)],
    old_agrees: bool,
}

const CLIP: &str = "Clip c:\n    Note/(";

const CASES: [Case; 34] = [
    Case {
        items: &["state x(3000000000)\n", "x"],
        drawn: &[(Code::DoesNotFit, "x")],
        old_agrees: true,
    },
    Case {
        items: &["state x(3000000000)\n", "x.past(1)"],
        drawn: &[(Code::DoesNotFit, "x.past(1)")],
        old_agrees: true,
    },
    Case {
        items: &["state x(3000000000)\n", "x.past(1, 0)"],
        drawn: &[(Code::DoesNotFit, "x.past(1, 0)")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([3000000000])\n", "xs[0]"],
        drawn: &[(Code::DoesNotFit, "xs[0]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([-3000000000, 1])\n", "xs[0]"],
        drawn: &[(Code::DoesNotFit, "xs[0]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([1, 3000000000])\n", "xs[1]"],
        drawn: &[(Code::DoesNotFit, "xs[1]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([1, 3000000000])\n", "xs[0]"],
        drawn: &[],
        old_agrees: true,
    },
    Case {
        items: &["state xs([1, 2])\n", "xs[1]"],
        drawn: &[],
        old_agrees: true,
    },
    Case {
        items: &["state i(0)\n", "state xs([1, 3000000000])\n", "xs[i]"],
        drawn: &[(Code::DoesNotFit, "xs")],
        old_agrees: true,
    },
    Case {
        items: &["state i(0)\n", "state xs([1, 2])\n", "xs[i]"],
        drawn: &[],
        old_agrees: true,
    },
    Case {
        items: &["state xs([1, 3000000000])\n", "xs[^1]"],
        drawn: &[(Code::DoesNotFit, "xs[^1]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([1, 3000000000])\n", "xs[0..1][0]"],
        drawn: &[],
        old_agrees: true,
    },
    Case {
        items: &["state xs([1, 3000000000])\n", "xs[1..][0]"],
        drawn: &[(Code::DoesNotFit, "xs[1..][0]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([[1, 2], [3, 3000000000]])\n", "xs[1][1]"],
        drawn: &[(Code::DoesNotFit, "xs[1][1]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([[1, 2], [3, 3000000000]])\n", "xs[1][0]"],
        drawn: &[],
        old_agrees: true,
    },
    Case {
        items: &["state xs([[1, 2], [3, 3000000000]])\n", "xs[^1][^1]"],
        drawn: &[(Code::DoesNotFit, "xs[^1][^1]")],
        old_agrees: true,
    },
    Case {
        items: &[
            "state i(0)\n",
            "state xs([[1, 2], [3, 3000000000]])\n",
            "xs[1][i]",
        ],
        drawn: &[(Code::DoesNotFit, "xs[1]")],
        old_agrees: true,
    },
    Case {
        items: &[
            "state i(0)\n",
            "state xs([[1, 2], [3, 3000000000]])\n",
            "xs[0][i]",
        ],
        drawn: &[],
        old_agrees: true,
    },
    Case {
        items: &[
            "state i(0)\n",
            "state xs([[1, 2], [3, 3000000000]])\n",
            "xs[i][0]",
        ],
        drawn: &[],
        old_agrees: true,
    },
    Case {
        items: &[
            "state i(0)\n",
            "state xs([[1], [3000000000]])\n",
            "xs[i][0]",
        ],
        drawn: &[(Code::DoesNotFit, "xs")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([[1, 2], [3, 3000000000]])\n", "xs[1][1..][0]"],
        drawn: &[(Code::DoesNotFit, "xs[1][1..][0]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([[1, 2], [3, 3000000000]])\n", "xs[0..2][1][1]"],
        drawn: &[(Code::DoesNotFit, "xs[0..2][1][1]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([[[1], [3000000000]]])\n", "xs[0][1][0]"],
        drawn: &[(Code::DoesNotFit, "xs[0][1][0]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([[1, 2], [3, 3000000000]])\n", "xs.past(1)[1][1]"],
        drawn: &[(Code::DoesNotFit, "xs.past(1)[1][1]")],
        old_agrees: false,
    },
    Case {
        items: &["state xs([[1, 2], [3, 3000000000]])\n", "xs.past(1)[0][0]"],
        drawn: &[],
        old_agrees: false,
    },
    Case {
        items: &["state xs([1, 3000000000])\n", "state y = xs[1]\n", "y"],
        drawn: &[(Code::DoesNotFit, "xs[1]")],
        old_agrees: true,
    },
    Case {
        items: &["state xs([1, 3000000000])\n", "state y = xs\n", "y[1]"],
        drawn: &[(Code::DoesNotFit, "xs")],
        old_agrees: true,
    },
    Case {
        items: &[
            "state xs([[1, 2], [3, 3000000000]])\n",
            "fn g(r): r[0]\n",
            "g(xs[1])",
        ],
        drawn: &[],
        old_agrees: true,
    },
    Case {
        items: &[
            "state xs([[1, 2], [3, 3000000000]])\n",
            "fn g(r): r\n",
            "g(xs)[1][1]",
        ],
        drawn: &[(Code::DoesNotFit, "g(xs)[1][1]")],
        old_agrees: true,
    },
    Case {
        items: &["state x(3000000000)\n", "fn f(v): v + 1\n", "f(x)"],
        drawn: &[(Code::DoesNotFit, "x")],
        old_agrees: true,
    },
    Case {
        items: &[
            "state xs([1, 3000000000])\n",
            "fn g(r): r[1] + r[0]\n",
            "g(xs)",
        ],
        drawn: &[(Code::DoesNotFit, "r[1]")],
        old_agrees: false,
    },
    Case {
        items: &[
            "state xs([[1, 2], [3, 3000000000]])\n",
            "if true then xs[0][0] else xs[1][1]",
        ],
        drawn: &[],
        old_agrees: false,
    },
    Case {
        items: &["state xs([[1, 2], [3, 3000000000]])\n", "xs[1][5]"],
        drawn: &[(Code::IndexOutOfRange, "5")],
        old_agrees: false,
    },
    Case {
        items: &["state xs([1, 3000000000])\n", "xs[1] * 0.5"],
        drawn: &[(Code::TypeMismatch, "xs[1] * 0.5")],
        old_agrees: false,
    },
];

fn orders(items: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    let mut order: Vec<usize> = (0..items.len()).collect();
    let mut counts = vec![0usize; items.len()];
    found.push(order.iter().map(|at| items[*at].as_str()).collect());
    let mut at = 0;
    while at < items.len() {
        if counts[at] < at {
            let other = if at % 2 == 0 { 0 } else { counts[at] };
            order.swap(other, at);
            found.push(order.iter().map(|at| items[*at].as_str()).collect());
            counts[at] += 1;
            at = 0;
        } else {
            counts[at] = 0;
            at += 1;
        }
    }
    found
}

fn shown(text: &str, found: &[Diagnostic]) -> Vec<(Code, String)> {
    let mut shown: Vec<(Code, String)> = found
        .iter()
        .filter(|diagnostic| diagnostic.code != Code::NoEntryInstance)
        .map(|diagnostic| {
            let range = diagnostic.span.range.start as usize..diagnostic.span.range.end as usize;
            (
                diagnostic.code,
                text.get(range).unwrap_or_default().to_string(),
            )
        })
        .collect();
    shown.sort_by(|a, b| (a.0.as_str(), &a.1).cmp(&(b.0.as_str(), &b.1)));
    shown
}

fn items_of(case: &Case) -> Vec<String> {
    let (read, rest) = case.items.split_last().expect("a case reads its cell");
    let mut items: Vec<String> = rest.iter().map(|item| (*item).to_string()).collect();
    items.push(format!("{CLIP}{read})\n"));
    items
}

#[test]
fn a_host_fed_cells_init_fits_every_use_element_by_element_at_each_read_in_every_order_495() {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let old = Compiler::new(&vocabulary)
        .expect("the vocabulary compiles")
        .on_the_old_engine();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let mut programs = 0;
    for case in &CASES {
        let mut wanted: Vec<(Code, String)> = case
            .drawn
            .iter()
            .map(|(code, text)| (*code, (*text).to_string()))
            .collect();
        wanted.sort_by(|a, b| (a.0.as_str(), &a.1).cmp(&(b.0.as_str(), &b.1)));
        for text in orders(&items_of(case)) {
            programs += 1;
            let found = new
                .checked_by_new_core(&[source(&text)])
                .unwrap_or_else(|| panic!("the new core takes {text}"));
            assert_eq!(shown(&text, &found), wanted, "{text}");
            let expected = old.check(&[source(&text)]);
            assert_eq!(found == expected, case.old_agrees, "{text}\n  {expected:?}");
        }
    }
    assert_eq!(programs, 116);
}

#[test]
fn a_host_fed_array_cells_element_read_through_a_parameter_draws_at_the_bodys_read_once_per_outermost_call_495()
 {
    let vocabulary = symphonyscript_fixtures::vocabulary();
    let new = Compiler::new(&vocabulary).expect("the vocabulary compiles");
    let text = "state xs([[1, 2], [3, 3000000000]])\nfn g(r): r[1]\nClip c:\n    Note/(g(xs[1]))\n    Note/(g(xs[1]))\n";
    let found = new
        .checked_by_new_core(&[source(text)])
        .expect("the new core takes it");
    let calls: Vec<Vec<String>> = found
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::DoesNotFit)
        .map(|diagnostic| {
            assert_eq!(
                text.get(diagnostic.span.range.start as usize..diagnostic.span.range.end as usize),
                Some("r[1]")
            );
            diagnostic
                .related
                .iter()
                .map(|related| {
                    let range = related.span.range.start as usize..related.span.range.end as usize;
                    text.get(range).unwrap_or_default().to_string()
                })
                .collect()
        })
        .collect();
    assert_eq!(calls.len(), 2, "{found:?}");
    assert!(
        calls.iter().all(|call| call == &["g(xs[1])".to_string()]),
        "{calls:?}"
    );
}
