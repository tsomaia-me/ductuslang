use super::grouped;

#[test]
fn a_bound_prints_with_its_digits_grouped_by_thousands() {
    for (value, expected) in [
        (0, "0"),
        (10, "10"),
        (999, "999"),
        (1_000, "1,000"),
        (1_024, "1,024"),
        (99_999, "99,999"),
        (100_000, "100,000"),
        (1_000_000, "1,000,000"),
        (2_000_000, "2,000,000"),
        (4_294_967_296, "4,294,967,296"),
        (u64::MAX, "18,446,744,073,709,551,615"),
    ] {
        assert_eq!(grouped(value), expected, "{value}");
    }
}

fn drawn(
    file: &str,
    help: Option<&str>,
    related: &[(&str, u32)],
) -> crate::diagnostics::Diagnostic {
    let mut diagnostic = crate::diagnostics::Diagnostic::new(
        crate::diagnostics::Code::OperandType,
        symphonyscript_core::Span {
            file: file.to_string(),
            range: 4..9,
        },
        "`+` takes two numbers or two strings",
    );
    diagnostic.help = help.map(str::to_string);
    diagnostic.related = related
        .iter()
        .map(|(file, start)| crate::diagnostics::Related {
            span: symphonyscript_core::Span {
                file: (*file).to_string(),
                range: *start..start + 1,
            },
            label: "in this call".to_string(),
        })
        .collect();
    diagnostic
}

#[test]
fn diagnostics_order_by_help_and_by_related_files_in_top_level_order_and_an_exact_pair_is_one() {
    let files = ["z.sym".to_string(), "a.sym".to_string()];
    let rows: [(
        Vec<crate::diagnostics::Diagnostic>,
        Vec<crate::diagnostics::Diagnostic>,
    ); 3] = [
        (
            vec![
                drawn("z.sym", Some("widen it"), &[]),
                drawn("z.sym", Some("convert it"), &[]),
            ],
            vec![
                drawn("z.sym", Some("convert it"), &[]),
                drawn("z.sym", Some("widen it"), &[]),
            ],
        ),
        (
            vec![
                drawn("z.sym", Some("convert it"), &[]),
                drawn("z.sym", Some("convert it"), &[]),
            ],
            vec![drawn("z.sym", Some("convert it"), &[])],
        ),
        (
            vec![
                drawn("z.sym", None, &[("a.sym", 0)]),
                drawn("z.sym", None, &[("z.sym", 7)]),
            ],
            vec![
                drawn("z.sym", None, &[("z.sym", 7)]),
                drawn("z.sym", None, &[("a.sym", 0)]),
            ],
        ),
    ];
    for (drawn_in, expected) in rows {
        for reversed in [false, true] {
            let mut report = super::Report::default();
            let mut ordered = drawn_in.clone();
            if reversed {
                ordered.reverse();
            }
            report.diagnostics = ordered;
            assert_eq!(report.finish(&files), expected, "reversed {reversed}");
        }
    }
}
