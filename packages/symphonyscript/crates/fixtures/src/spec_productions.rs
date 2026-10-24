use crate::spec_examples::spec_examples;

pub fn spec_productions() -> Vec<String> {
    spec_examples()
        .iter()
        .filter(|example| example.tag == "ebnf")
        .flat_map(|example| {
            example
                .text
                .lines()
                .filter_map(|line| {
                    let (name, _) = line.split_once("::=")?;
                    let name = name.trim();
                    (!name.is_empty() && !line.starts_with(' ')).then(|| name.to_string())
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
