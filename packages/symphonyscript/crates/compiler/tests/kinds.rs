mod support;

use symphonyscript::{Code, Compiler};
use symphonyscript_fixtures::{Orders, Rows, covered, vocabulary};

const BASES: [(&str, &str, &str, &str); 4] = [
    ("", "[1.5, 2.5]", "", "0"),
    (
        "input f = [1.5, 2.5]\n",
        "f",
        "input f = [1.5, 2.5]\ninput n = 0\n",
        "n",
    ),
    ("", "[1, 2]", "", "0"),
    (
        "input g = [1, 2]\n",
        "g",
        "input g = [1, 2]\ninput n = 0\n",
        "n",
    ),
];

const INDEXES: [&str; 3] = ["true", "1.5", "Wave::Saw"];

const CONSUMERS: [&str; 4] = [
    "Note/(R)",
    "Note/(R + 1.5)",
    "Note when R > 1",
    "Note/(i32(R))",
];

fn index_rows(rows: &Rows, orders: Orders) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (base_tag, (declaration, base, twin_declaration, twin_index)) in BASES.iter().enumerate() {
        for (index_tag, index) in INDEXES.iter().enumerate() {
            for (consumer_tag, consumer) in CONSUMERS.iter().enumerate() {
                let clip = |read: &str| format!("Clip c:\n    {}\n", consumer.replace('R', read));
                let faulty = clip(&format!("{base}[{index}]"));
                let valid = clip(&format!("{base}[{twin_index}]"));
                let items: Vec<&str> = [*declaration, faulty.as_str()]
                    .into_iter()
                    .filter(|item| !item.is_empty())
                    .collect();
                let twins: Vec<&str> = [*twin_declaration, valid.as_str()]
                    .into_iter()
                    .filter(|item| !item.is_empty())
                    .collect();
                for (order_tag, (order, twin)) in orders
                    .of(&items)
                    .into_iter()
                    .zip(orders.of(&twins))
                    .enumerate()
                {
                    if !rows.take(&[base_tag, index_tag, consumer_tag, order_tag]) {
                        continue;
                    }
                    programs += 1;
                    let text = order.concat();
                    let found: Vec<Code> = support::error_texts(&compiler, &text)
                        .into_iter()
                        .map(|(code, _)| code)
                        .collect();
                    let mut expected: Vec<Code> = support::error_texts(&compiler, &twin.concat())
                        .into_iter()
                        .map(|(code, _)| code)
                        .collect();
                    expected.push(Code::IndexType);
                    expected.sort();
                    if found != expected {
                        failures.push(format!(
                            "{text}\n  found {found:?}\n  expected {expected:?}\n"
                        ));
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn assert_none_fail((failures, programs): (Vec<String>, usize), expected: usize) {
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(8).cloned().collect::<String>()
    );
    assert_eq!(programs, expected);
}

#[test]
fn an_element_read_whose_compile_time_index_is_in_error_has_its_elements_type() {
    assert_none_fail(
        covered(|rows| index_rows(rows, Orders::ForwardAndReversed)),
        17,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_element_read_whose_compile_time_index_is_in_error_has_its_elements_type() {
    assert_none_fail(index_rows(&Rows::Every, Orders::Every), 72);
}
