mod support;

use symphonyscript::{Code, Compiler, Diagnostic, Source};
use symphonyscript_fixtures::{
    Project, dsp_project, music_project, pending_spec_examples, spec_projects, vocabulary,
};

fn projects() -> Vec<Project> {
    let mut projects = vec![music_project(), dsp_project()];
    let pending = pending_spec_examples();
    projects.extend(
        spec_projects()
            .into_iter()
            .filter(|project| !pending.contains(&project.name)),
    );
    projects
}

fn items(text: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for line in text.split_inclusive('\n') {
        let starts_item = line
            .chars()
            .next()
            .is_some_and(|first| !first.is_whitespace())
            && !line.starts_with("//");
        match items.last_mut() {
            Some(item) if !starts_item => item.push_str(line),
            _ => items.push(line.to_string()),
        }
    }
    items
}

fn errors(sources: &[Source]) -> Vec<Diagnostic> {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    support::errors(compiler.check(sources))
}

fn sources(project: &Project) -> Vec<Source> {
    project
        .files
        .iter()
        .map(|file| Source {
            path: file.path.clone(),
            text: file.text.clone(),
        })
        .collect()
}

#[test]
fn top_level_declarations_refer_to_each_other_regardless_of_order() {
    for project in projects() {
        let reversed: Vec<Source> = sources(&project)
            .into_iter()
            .map(|source| Source {
                text: items(&source.text).into_iter().rev().collect(),
                ..source
            })
            .collect();
        assert_eq!(errors(&reversed), vec![], "{}", project.name);
    }
}

#[test]
fn files_share_one_namespace() {
    for project in projects() {
        let split: Vec<Source> = sources(&project)
            .iter()
            .flat_map(|source| items(&source.text))
            .enumerate()
            .map(|(position, text)| Source {
                path: format!("part{position:03}.sym"),
                text,
            })
            .collect();
        assert_eq!(errors(&split), vec![], "{}", project.name);
    }
}

#[test]
fn a_second_declaration_of_a_name_is_exactly_one_duplicate_error() {
    let mut cases = 0;
    for project in projects() {
        for source in sources(&project) {
            let parts = items(&source.text);
            for (position, part) in parts.iter().enumerate() {
                let declaration = ["input ", "let ", "fn ", "pattern "]
                    .iter()
                    .any(|keyword| part.starts_with(keyword));
                if !declaration {
                    continue;
                }
                cases += 1;
                let mut text: String = parts.concat();
                text.push_str(part);
                let found = errors(&[Source {
                    path: source.path.clone(),
                    text: text.clone(),
                }]);
                let codes: Vec<Code> = found.iter().map(|diagnostic| diagnostic.code).collect();
                assert_eq!(
                    codes,
                    vec![Code::DuplicateName],
                    "{} item {position}: {found:?}",
                    project.name
                );
                assert!(
                    found[0].span.range.start as usize >= parts.concat().len(),
                    "{} item {position}: the error points at the copy",
                    project.name
                );
            }
        }
    }
    assert!(cases > 20, "{cases} declarations duplicated");
}

#[test]
fn a_value_passed_through_a_function_keeps_its_meaning() {
    let mut wraps = 0;
    for project in projects() {
        let wrapped: Vec<Source> = sources(&project)
            .into_iter()
            .enumerate()
            .map(|(position, source)| {
                let mut text: String = items(&source.text)
                    .into_iter()
                    .map(|item| match item.strip_prefix("let ") {
                        Some(rest)
                            if !item.contains("match") && item.trim_end().lines().count() == 1 =>
                        {
                            match rest.split_once(" = ") {
                                Some((name, value)) => {
                                    wraps += 1;
                                    let trailing = &item[item.trim_end().len()..];
                                    format!("let {name} = same({}){trailing}", value.trim_end())
                                }
                                None => item,
                            }
                        }
                        _ => item,
                    })
                    .collect();
                if position == 0 {
                    text.push_str("fn same(v): v\n");
                }
                Source { text, ..source }
            })
            .collect();
        assert_eq!(errors(&wrapped), vec![], "{}", project.name);
    }
    assert!(wraps > 5, "{wraps} values wrapped");
}

fn orders(sources: &[Source]) -> Vec<Vec<Source>> {
    let mut current: Vec<Source> = sources.to_vec();
    let mut counters = vec![0; current.len()];
    let mut found = vec![current.clone()];
    let mut position = 1;
    while position < current.len() {
        if counters[position] < position {
            let other = if position % 2 == 0 {
                0
            } else {
                counters[position]
            };
            current.swap(other, position);
            found.push(current.clone());
            counters[position] += 1;
            position = 1;
        } else {
            counters[position] = 0;
            position += 1;
        }
    }
    found
}

fn files(files: &[(&str, &str)]) -> Vec<Source> {
    files
        .iter()
        .map(|(path, text)| Source {
            path: (*path).into(),
            text: (*text).into(),
        })
        .collect()
}

#[test]
fn every_order_of_the_sources_gives_the_same_diagnostics() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let mut programs: Vec<Vec<Source>> = projects().iter().map(sources).collect();
    programs.push(files(&[("b.sym", "let x = 1\n"), ("a.sym", "let x = 2\n")]));
    programs.push(files(&[
        ("d/e.sym", "Clip c:\n    Note\n"),
        ("a.sym", "let p = q\nClip lonely:\n    Note\n"),
        ("c.sym", "let p = 1\n"),
        ("b.sym", "Clip c:\n    Note/3000000000\n"),
    ]));
    let mut checked = 0;
    for program in programs {
        let expected = compiler.check(&program);
        for order in orders(&program) {
            assert_eq!(compiler.check(&order), expected, "{order:?}");
            checked += 1;
        }
    }
    assert!(checked > 24, "{checked} orders checked");
}
