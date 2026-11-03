mod fault_catalogue;
mod generated_file;
mod kind_reads;
mod line_shape;
mod repeated_faults;
mod single_faults;
mod valid_items;

use symphonyscript_core::Vocabulary;

pub(crate) use kind_reads::{NESTS, Nest, nest_program};

pub(crate) fn item_programs(vocabulary: &Vocabulary) -> Vec<String> {
    let faults = fault_catalogue::fault_items();
    let valid = valid_items::valid_items(vocabulary);
    let mut texts: Vec<String> = faults.iter().map(|item| item.text.clone()).collect();
    texts.extend(valid.top.iter().cloned());
    texts.extend(
        valid
            .body
            .iter()
            .map(|line| format!("{}{line}", valid_items::HOST)),
    );
    texts.extend(kind_reads::nest_programs(64));
    texts
}

pub(crate) fn generated_programs(vocabulary: &Vocabulary) -> Vec<String> {
    let faults = fault_catalogue::fault_items();
    let valid = valid_items::valid_items(vocabulary);
    let pool = generated_file::Pool::new(&faults, &valid, vocabulary);
    (1..=single_faults::SEEDS)
        .map(|seed| generated_file::generate(seed, faults.len(), &pool).text)
        .collect()
}

pub(crate) fn nest_programs(depth: usize) -> Vec<String> {
    kind_reads::nest_programs(depth)
}
