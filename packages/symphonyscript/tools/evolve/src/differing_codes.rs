use std::collections::BTreeMap;

pub fn differing_codes(baseline: &[String], variant: &[String]) -> Vec<String> {
    let mut balance: BTreeMap<&str, i64> = BTreeMap::new();
    for code in baseline {
        *balance.entry(code).or_default() += 1;
    }
    for code in variant {
        *balance.entry(code).or_default() -= 1;
    }
    balance
        .into_iter()
        .filter(|(_, count)| *count != 0)
        .map(|(code, _)| code.to_string())
        .collect()
}
