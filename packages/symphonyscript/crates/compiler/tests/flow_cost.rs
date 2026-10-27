use symphonyscript::{Code, Compiler, Severity, Source};
use symphonyscript_core::{Count, Vocabulary};

fn vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("flow cost");
    let wave = v.enumeration("Wave", ["Sine", "Saw"]);
    let color = v.enumeration("Color", ["Red", "Blue"]);
    let both = v.node("K");
    v.kind(both)
        .top_level(Count::Any)
        .attr_enum("wave", &wave, "Sine")
        .attr_enum("color", &color, "Red");
    v.build().expect("the flow cost vocabulary builds")
}

fn program(uses: usize) -> String {
    let mut text =
        String::from("pattern P():\n    let a = Sine\n    let b = a\n    K | wave=a color=b\n");
    for use_ in 0..uses {
        text.push_str(&format!("P p{use_}\n"));
    }
    text
}

fn checked(compiler: &Compiler, text: &str) -> Vec<(Code, String, Vec<String>)> {
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let span = |range: std::ops::Range<u32>| {
        let at = usize::try_from(range.start).unwrap_or(0)..usize::try_from(range.end).unwrap_or(0);
        text.get(at).unwrap_or("").to_string()
    };
    compiler
        .check(&[source])
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .map(|diagnostic| {
            let mut secondary: Vec<String> = diagnostic
                .related
                .iter()
                .map(|related| &related.span)
                .map(|loc| span(loc.range.clone()))
                .collect();
            secondary.sort();
            (
                diagnostic.code,
                span(diagnostic.span.range.clone()),
                secondary,
            )
        })
        .collect()
}

#[test]
fn erroneous_flows_report_one_diagnostic_per_use() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for uses in [2000, 8000] {
        let errors = checked(&compiler, &program(uses));
        assert_eq!(errors.len(), uses, "one diagnostic per use");
        assert_eq!(
            errors.first(),
            Some(&(
                Code::TypeMismatch,
                "Sine".to_string(),
                vec!["P p0".to_string(), "let b = a\n".to_string()]
            ))
        );
    }
}
