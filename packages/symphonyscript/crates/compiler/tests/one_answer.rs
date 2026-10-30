use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MODULE: &str = "analysis/answers/";

const DECIDING_NONE: &[&str] = &[
    "analysis/evaluation/run.rs compute",
    "analysis/evaluation/run.rs fit",
    "analysis/evaluation/run.rs takes_a_type",
    "analysis/evaluation/run.rs takes_expected",
    "analysis/expand/structure.rs range_bound",
    "analysis/infer/check.rs check_from",
    "analysis/infer/check.rs destination_labels",
    "analysis/infer/check.rs own_operands",
    "analysis/infer/expectations.rs expect",
    "analysis/infer/positions.rs arm_position",
    "analysis/infer/positions.rs child_flow",
    "analysis/infer/positions.rs chosen_child",
    "analysis/infer/positions.rs detached_roots",
    "analysis/infer/positions.rs picked_element",
    "analysis/infer/units.rs carried_operands",
    "analysis/infer/units.rs read_in_units_of",
    "analysis/infer/values.rs retract_unread_evaluations",
    "analysis/infer/widths.rs natural_parts",
    "analysis/op.rs choice_arms",
    "analysis/op.rs operands",
    "analysis/resolve/calls.rs same_path",
    "analysis/resolve/declarations.rs holds_no_instance",
    "analysis/resolve/expressions.rs match_val",
    "analysis/resolve/expressions.rs run_expr",
    "analysis/resolve/names.rs unplaced_placeholder",
    "analysis/resolve/names.rs unplaced_variant",
    "analysis/resolve/paths.rs bound_slice",
    "analysis/resolve/paths.rs candidates",
    "analysis/resolve/paths.rs chosen_read",
    "analysis/resolve/paths.rs field",
    "analysis/resolve/paths.rs reference_instances",
    "analysis/resolve/paths.rs to_path",
    "analysis/resolve/paths.rs written_base",
    "analysis/resolve/paths.rs written_destination",
    "analysis/resolve/selection.rs pass_over",
    "analysis/resolve/selection.rs selected",
    "ir/builder/expressions.rs negation",
    "ir/expr_kind.rs children",
    "scopes/checker.rs expr",
];

const STILL_DECIDING: &[(&str, &str)] = &[
    ("analysis/carried_enum_of.rs carried_enum_of", "kind"),
    ("analysis/infer/array_checks.rs is_string_bytes", "kind"),
    ("analysis/infer/bare_scrutinees.rs bare_scrutinees", "kind"),
    ("analysis/infer/capacities.rs combined_extent", "shape"),
    ("analysis/infer/capacities.rs cover", "shape"),
    ("analysis/infer/capacities.rs extent_parts", "shape"),
    (
        "analysis/infer/capacities.rs state_dependencies",
        "reads a cell",
    ),
    ("analysis/infer/check.rs holds_a_float_cell", "kind"),
    ("analysis/infer/check.rs native_argument", "shape"),
    (
        "analysis/infer/compile_time_enums.rs compile_time_enums",
        "kind",
    ),
    ("analysis/infer/compile_time_enums.rs enum_kind", "kind"),
    ("analysis/infer/enum_bindings.rs carried_enum", "kind"),
    ("analysis/infer/enum_bindings.rs member_of", "kind"),
    ("analysis/infer/enum_bindings.rs use_end", "kind"),
    ("analysis/infer/enum_bindings.rs value_bares", "kind"),
    ("analysis/infer/enum_bindings.rs value_parts", "kind"),
    (
        "analysis/infer/enum_bindings.rs value_reads",
        "reads a cell",
    ),
    ("analysis/infer/enum_bindings.rs writes_variant", "kind"),
    ("analysis/infer/fixedness.rs group_fallbacks", "shape"),
    (
        "analysis/infer/fixedness.rs scan_state_reads",
        "reads a cell",
    ),
    ("analysis/infer/history_reads.rs kind_edges", "kind"),
    ("analysis/infer/history_reads.rs settle_kinds", "kind"),
    ("analysis/infer/order.rs reads_of", "reads a cell"),
    ("analysis/infer/synth.rs synth", "kind"),
    ("analysis/infer/synth.rs synth_of", "kind"),
    ("analysis/resolve/arrays.rs bare_leaves", "kind"),
    ("analysis/resolve/arrays.rs choice_bytes", "shape"),
    ("analysis/resolve/arrays.rs reaches_choice", "shape"),
    ("analysis/resolve/calls.rs arm_parts", "kind"),
    ("analysis/resolve/calls.rs combined_arm_type", "kind"),
    ("analysis/resolve/history.rs history_reads", "reads a cell"),
    ("analysis/scrutinee_ty.rs choice_ty", "kind"),
    ("analysis/signature/matches.rs label", "kind"),
    ("analysis/signature/run.rs joins", "kind"),
    ("analysis/signature/run.rs reaches_recursion", "shape"),
    ("analysis/signature/run.rs run_sig", "kind"),
];

const CATCH_ALLS_OUTSIDE: &[&str] = &[
    "analysis/carried_enum_of.rs carried_enum_of",
    "analysis/infer/array_checks.rs is_string_bytes",
    "analysis/infer/bare_scrutinees.rs bare_scrutinees",
    "analysis/infer/capacities.rs combined_extent",
    "analysis/infer/capacities.rs cover",
    "analysis/infer/capacities.rs extent_parts",
    "analysis/infer/capacities.rs state_dependencies",
    "analysis/infer/check.rs check_from",
    "analysis/infer/check.rs destination_labels",
    "analysis/infer/check.rs holds_a_float_cell",
    "analysis/infer/check.rs native_argument",
    "analysis/infer/check.rs own_operands",
    "analysis/infer/compile_time_enums.rs compile_time_enums",
    "analysis/infer/compile_time_enums.rs enum_kind",
    "analysis/infer/enum_bindings.rs carried_enum",
    "analysis/infer/enum_bindings.rs member_of",
    "analysis/infer/enum_bindings.rs use_end",
    "analysis/infer/enum_bindings.rs value_bares",
    "analysis/infer/enum_bindings.rs value_reads",
    "analysis/infer/enum_bindings.rs writes_variant",
    "analysis/infer/expectations.rs expect",
    "analysis/infer/fixedness.rs group_fallbacks",
    "analysis/infer/fixedness.rs scan_state_reads",
    "analysis/infer/history_reads.rs kind_edges",
    "analysis/infer/history_reads.rs settle_kinds",
    "analysis/infer/order.rs reads_of",
    "analysis/infer/positions.rs detached_roots",
    "analysis/infer/positions.rs picked_element",
    "analysis/infer/synth.rs synth",
    "analysis/infer/units.rs carried_operands",
    "analysis/infer/widths.rs natural_parts",
    "analysis/resolve/arrays.rs bare_leaves",
    "analysis/resolve/arrays.rs choice_bytes",
    "analysis/resolve/arrays.rs reaches_choice",
    "analysis/resolve/calls.rs arm_parts",
    "analysis/resolve/calls.rs same_path",
    "analysis/resolve/declarations.rs holds_no_instance",
    "analysis/resolve/expressions.rs match_val",
    "analysis/resolve/history.rs history_reads",
    "analysis/resolve/names.rs unplaced_placeholder",
    "analysis/resolve/paths.rs chosen_read",
    "analysis/resolve/paths.rs to_path",
    "analysis/resolve/paths.rs written_base",
    "analysis/resolve/paths.rs written_destination",
    "analysis/resolve/selection.rs pass_over",
    "analysis/resolve/selection.rs selected",
    "analysis/signature/matches.rs label",
    "analysis/signature/run.rs reaches_recursion",
    "scopes/checker.rs expr",
];

#[derive(Debug)]
struct Matched {
    site: String,
    catch_all: bool,
}

fn sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let entries = std::fs::read_dir(&folder).expect("the source folder reads");
        for entry in entries {
            let path = entry.expect("the entry reads").path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn skip_literal(bytes: &[u8], at: usize) -> usize {
    match bytes[at] {
        b'"' => {
            let mut position = at + 1;
            while position < bytes.len() && bytes[position] != b'"' {
                position += if bytes[position] == b'\\' { 2 } else { 1 };
            }
            position + 1
        }
        b'\'' if bytes.get(at + 1) == Some(&b'\\') => {
            let mut position = at + 2;
            while position < bytes.len() && bytes[position] != b'\'' {
                position += 1;
            }
            position + 1
        }
        b'\'' if bytes.get(at + 2) == Some(&b'\'') => at + 3,
        _ => at + 1,
    }
}

fn names_enum(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let starts = |needle: &str| {
        pattern.match_indices(needle).any(|(at, _)| {
            at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_')
        })
    };
    starts("ExprKind::") || starts("Op::")
}

fn catches_all(pattern: &str) -> bool {
    let first = pattern
        .split(" if ")
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches('|')
        .trim();
    first == "_"
        || first
            .split('|')
            .any(|alternative| alternative.trim() == "_")
        || (!first.is_empty()
            && first
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_'))
}

fn arms(bytes: &[u8], open: usize) -> (Vec<String>, usize) {
    let mut patterns = Vec::new();
    let (mut braces, mut parens) = (1i32, 0i32);
    let mut position = open + 1;
    let mut start = position;
    let mut in_body = false;
    let mut block_body = false;
    while position < bytes.len() {
        let byte = bytes[position];
        if byte == b'"' || byte == b'\'' {
            position = skip_literal(bytes, position);
            continue;
        }
        match byte {
            b'{' => braces += 1,
            b'(' | b'[' => parens += 1,
            b')' | b']' => parens -= 1,
            b'}' => {
                braces -= 1;
                if braces == 0 {
                    return (patterns, position);
                }
                if in_body && block_body && braces == 1 {
                    in_body = false;
                    position += 1;
                    while position < bytes.len()
                        && (bytes[position] == b',' || bytes[position].is_ascii_whitespace())
                    {
                        position += 1;
                    }
                    start = position;
                    continue;
                }
            }
            b'=' if !in_body
                && braces == 1
                && parens == 0
                && bytes.get(position + 1) == Some(&b'>') =>
            {
                patterns.push(
                    String::from_utf8_lossy(&bytes[start..position])
                        .trim()
                        .to_string(),
                );
                in_body = true;
                position += 2;
                while position < bytes.len() && bytes[position].is_ascii_whitespace() {
                    position += 1;
                }
                block_body = bytes.get(position) == Some(&b'{');
                continue;
            }
            b',' if in_body && !block_body && braces == 1 && parens == 0 => {
                in_body = false;
                start = position + 1;
            }
            _ => {}
        }
        position += 1;
    }
    (patterns, position)
}

fn matches_in(text: &str, file: &str) -> Vec<Matched> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut function = String::from("<file>");
    let mut position = 0;
    while position < bytes.len() {
        let byte = bytes[position];
        if byte == b'"' || byte == b'\'' {
            position = skip_literal(bytes, position);
            continue;
        }
        let word_start = position == 0
            || !(bytes[position - 1].is_ascii_alphanumeric() || bytes[position - 1] == b'_');
        if word_start && bytes[position..].starts_with(b"fn ") {
            let name: Vec<u8> = bytes[position + 3..]
                .iter()
                .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'_')
                .copied()
                .collect();
            function = String::from_utf8_lossy(&name).to_string();
        }
        if word_start && bytes[position..].starts_with(b"match ") {
            let mut open = position + 6;
            let mut parens = 0i32;
            while open < bytes.len() && !(bytes[open] == b'{' && parens == 0) {
                if bytes[open] == b'"' || bytes[open] == b'\'' {
                    open = skip_literal(bytes, open);
                    continue;
                }
                match bytes[open] {
                    b'(' | b'[' => parens += 1,
                    b')' | b']' => parens -= 1,
                    _ => {}
                }
                open += 1;
            }
            let (patterns, _) = arms(bytes, open);
            if patterns.iter().any(|pattern| names_enum(pattern)) {
                found.push(Matched {
                    site: format!("{file} {function}"),
                    catch_all: patterns.iter().any(|pattern| catches_all(pattern)),
                });
            }
        }
        position += 1;
    }
    found
}

fn inventory() -> Vec<Matched> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/semantics");
    let mut found = Vec::new();
    for path in sources(&root) {
        let file = path
            .strip_prefix(&root)
            .expect("the file is under the root")
            .to_string_lossy()
            .replace('\\', "/");
        if file.ends_with("tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("the source reads");
        found.extend(matches_in(&text, &file));
    }
    found
}

fn listed(sites: impl IntoIterator<Item = &'static str>) -> BTreeSet<String> {
    sites.into_iter().map(str::to_string).collect()
}

#[test]
fn every_match_over_an_expression_or_an_op_outside_the_answers_module_is_listed_with_what_it_decides()
 {
    let found = inventory();
    let outside: BTreeSet<String> = found
        .iter()
        .filter(|matched| !matched.site.starts_with(MODULE))
        .map(|matched| matched.site.clone())
        .collect();
    let none = listed(DECIDING_NONE.iter().copied());
    let still = listed(STILL_DECIDING.iter().map(|(site, _)| *site));
    assert!(none.is_disjoint(&still), "a site listed twice");
    let classified: BTreeSet<String> = none.union(&still).cloned().collect();
    let unlisted: Vec<&String> = outside.difference(&classified).collect();
    let gone: Vec<&String> = classified.difference(&outside).collect();
    assert!(
        unlisted.is_empty() && gone.is_empty(),
        "a match outside {MODULE} that no list names: {unlisted:#?}\na listed match that is gone: {gone:#?}"
    );
}

#[test]
fn no_match_over_an_expression_or_an_op_in_the_answers_module_has_a_catch_all_arm() {
    let found = inventory();
    let inside: Vec<&String> = found
        .iter()
        .filter(|matched| matched.catch_all && matched.site.starts_with(MODULE))
        .map(|matched| &matched.site)
        .collect();
    assert!(
        inside.is_empty(),
        "a catch-all arm in {MODULE}: {inside:#?}"
    );
    let outside: BTreeSet<String> = found
        .iter()
        .filter(|matched| matched.catch_all && !matched.site.starts_with(MODULE))
        .map(|matched| matched.site.clone())
        .collect();
    assert_eq!(outside, listed(CATCH_ALLS_OUTSIDE.iter().copied()));
}
