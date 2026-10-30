use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MODULE: &str = "analysis/answers/";

const DECIDING_NONE: &[(&str, &str)] = &[
    (
        "analysis/carried_enum_of.rs carried_enum_of",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/evaluation/run.rs compute",
        "the compile-time value of a node, evaluated by SPEC §4.6",
    ),
    (
        "analysis/evaluation/run.rs fit",
        "whether a compile-time value fits its type, SPEC §4.1's fit check",
    ),
    (
        "analysis/evaluation/run.rs takes_a_type",
        "the expected type that flows into an operand, SPEC §4.2",
    ),
    (
        "analysis/evaluation/run.rs takes_expected",
        "the expected type that flows into an operand, SPEC §4.2",
    ),
    (
        "analysis/expand/structure.rs range_bound",
        "a range bound written as a literal, read before evaluation",
    ),
    (
        "analysis/infer/bare_scrutinees.rs bare_scrutinees",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/capacities.rs combined_extent",
        "a variable array's capacity by SPEC §3.7's and §6.6's equations, over the module's shapes",
    ),
    (
        "analysis/infer/capacities.rs cover",
        "a variable array's capacity by SPEC §3.7's and §6.6's equations, over the module's shapes",
    ),
    (
        "analysis/infer/capacities.rs extent_parts",
        "a variable array's capacity by SPEC §3.7's and §6.6's equations, over the module's shapes",
    ),
    (
        "analysis/infer/check.rs check_from",
        "the diagnostics of Check against the types Infer settled",
    ),
    (
        "analysis/infer/check.rs destination_labels",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/infer/check.rs native_argument",
        "the element nodes of an array literal passed to a native, for the argument check",
    ),
    (
        "analysis/infer/check.rs own_operands",
        "where a diagnostic stands beside a fault, ARCHITECTURE §7's placeholder rule",
    ),
    (
        "analysis/infer/compile_time_enums.rs compile_time_enums",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/compile_time_enums.rs enum_kind",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/enum_bindings.rs carried_enum",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/enum_bindings.rs member_of",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/enum_bindings.rs use_end",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/enum_bindings.rs value_bares",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/enum_bindings.rs value_parts",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/enum_bindings.rs writes_variant",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/infer/expectations.rs expect",
        "the expected type that flows into an operand, SPEC §4.2",
    ),
    (
        "analysis/infer/fixedness.rs group_fallbacks",
        "which history fallbacks settle together, reading the module's layout",
    ),
    (
        "analysis/infer/positions.rs arm_position",
        "where a diagnostic stands beside a fault, ARCHITECTURE §7's placeholder rule",
    ),
    (
        "analysis/infer/positions.rs child_flow",
        "the expected type that flows into an operand, SPEC §4.2",
    ),
    (
        "analysis/infer/positions.rs chosen_child",
        "where a diagnostic stands beside a fault, ARCHITECTURE §7's placeholder rule",
    ),
    (
        "analysis/infer/positions.rs detached_roots",
        "where a diagnostic stands beside a fault, ARCHITECTURE §7's placeholder rule",
    ),
    (
        "analysis/infer/positions.rs picked_element",
        "where a diagnostic stands beside a fault, ARCHITECTURE §7's placeholder rule",
    ),
    (
        "analysis/infer/units.rs carried_operands",
        "the unit family a value carries, SPEC §3.6",
    ),
    (
        "analysis/infer/units.rs read_in_units_of",
        "the unit family a value carries, SPEC §3.6",
    ),
    (
        "analysis/infer/values.rs retract_unread_evaluations",
        "which compile-time evaluations no read keeps, for their diagnostics",
    ),
    (
        "analysis/op.rs choice_arms",
        "the operands of a node, the structure every walk follows",
    ),
    (
        "analysis/op.rs operands",
        "the operands of a node, the structure every walk follows",
    ),
    (
        "analysis/resolve/arrays.rs bare_leaves",
        "the enum a bare variant takes from its expected type, SPEC §4.2's enum flow",
    ),
    (
        "analysis/resolve/arrays.rs choice_bytes",
        "the byte array a choice of strings builds, SPEC §3.3",
    ),
    (
        "analysis/resolve/arrays.rs reaches_choice",
        "the byte array a choice of strings builds, SPEC §3.3",
    ),
    (
        "analysis/resolve/calls.rs same_path",
        "whether two argument expressions write the same name or path",
    ),
    (
        "analysis/resolve/expressions.rs match_val",
        "the node the resolver builds for each expression form",
    ),
    (
        "analysis/resolve/expressions.rs run_expr",
        "the node the resolver builds for each expression form",
    ),
    (
        "analysis/resolve/names.rs unplaced_placeholder",
        "the name a bare-variant placeholder prints in its diagnostic",
    ),
    (
        "analysis/resolve/names.rs unplaced_variant",
        "the name a bare-variant placeholder prints in its diagnostic",
    ),
    (
        "analysis/resolve/paths.rs bound_slice",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/resolve/paths.rs candidates",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/resolve/paths.rs chosen_read",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/resolve/paths.rs field",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/resolve/paths.rs reference_instances",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/resolve/paths.rs to_path",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/resolve/paths.rs written_base",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/resolve/paths.rs written_destination",
        "the instances a path or a wire destination names, SPEC §5.6 and §8.2",
    ),
    (
        "analysis/resolve/selection.rs pass_over",
        "which arm a compile-time choice takes, by evaluating its chooser",
    ),
    (
        "analysis/resolve/selection.rs selected",
        "which arm a compile-time choice takes, by evaluating its chooser",
    ),
    (
        "ir/builder/expressions.rs negation",
        "the literal a minus sign forms with a number, SPEC §4.1",
    ),
    (
        "ir/expr_kind.rs children",
        "the operands of a node, the structure every walk follows",
    ),
    (
        "scopes/checker.rs expr",
        "which names an expression uses, for scope checks",
    ),
];

const CATCH_ALLS_OUTSIDE: &[&str] = &[
    "analysis/carried_enum_of.rs carried_enum_of",
    "analysis/infer/bare_scrutinees.rs bare_scrutinees",
    "analysis/infer/capacities.rs combined_extent",
    "analysis/infer/capacities.rs cover",
    "analysis/infer/capacities.rs extent_parts",
    "analysis/infer/check.rs check_from",
    "analysis/infer/check.rs destination_labels",
    "analysis/infer/check.rs native_argument",
    "analysis/infer/check.rs own_operands",
    "analysis/infer/compile_time_enums.rs compile_time_enums",
    "analysis/infer/compile_time_enums.rs enum_kind",
    "analysis/infer/enum_bindings.rs carried_enum",
    "analysis/infer/enum_bindings.rs member_of",
    "analysis/infer/enum_bindings.rs use_end",
    "analysis/infer/enum_bindings.rs value_bares",
    "analysis/infer/enum_bindings.rs writes_variant",
    "analysis/infer/expectations.rs expect",
    "analysis/infer/fixedness.rs group_fallbacks",
    "analysis/infer/positions.rs detached_roots",
    "analysis/infer/positions.rs picked_element",
    "analysis/infer/units.rs carried_operands",
    "analysis/resolve/arrays.rs bare_leaves",
    "analysis/resolve/arrays.rs choice_bytes",
    "analysis/resolve/arrays.rs reaches_choice",
    "analysis/resolve/calls.rs same_path",
    "analysis/resolve/expressions.rs match_val",
    "analysis/resolve/names.rs unplaced_placeholder",
    "analysis/resolve/paths.rs chosen_read",
    "analysis/resolve/paths.rs to_path",
    "analysis/resolve/paths.rs written_base",
    "analysis/resolve/paths.rs written_destination",
    "analysis/resolve/selection.rs pass_over",
    "analysis/resolve/selection.rs selected",
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
fn every_match_over_an_expression_or_an_op_outside_the_answers_module_decides_another_question() {
    let found = inventory();
    let outside: BTreeSet<String> = found
        .iter()
        .filter(|matched| !matched.site.starts_with(MODULE))
        .map(|matched| matched.site.clone())
        .collect();
    let classified = listed(DECIDING_NONE.iter().map(|(site, _)| *site));
    assert_eq!(classified.len(), DECIDING_NONE.len(), "a site listed twice");
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
