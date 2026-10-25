use std::collections::{BTreeMap, BTreeSet};

use symphonyscript::Source;
use symphonyscript::syntax::{ElementView, SyntaxKind, Walk, parse};
use symphonyscript_core::{BUILTINS, KEYWORDS, RESERVED_WORDS, Vocabulary};
use unicode_normalization::UnicodeNormalization;

use crate::program::{Program, ProgramSource};

const DECLARATIONS: &[SyntaxKind] = &[
    SyntaxKind::InputDecl,
    SyntaxKind::LetDecl,
    SyntaxKind::FnDecl,
    SyntaxKind::PatternDecl,
    SyntaxKind::Param,
    SyntaxKind::LocalLet,
    SyntaxKind::PatternParam,
    SyntaxKind::Placement,
    SyntaxKind::ForBlock,
    SyntaxKind::BundleFor,
];

pub struct Renamed {
    pub program: Program,
    pub originals: BTreeMap<String, String>,
}

struct Identifiers {
    occurrences: Vec<Vec<(usize, usize, String)>>,
    declared: BTreeSet<String>,
    seen: BTreeSet<String>,
}

struct Naming {
    reserved: BTreeSet<String>,
    taken: BTreeSet<String>,
    counter: usize,
}

pub fn renamed(program: &Program, vocabulary: &Vocabulary) -> Renamed {
    let identifiers = identifiers(program, vocabulary);
    let fresh = Naming::new(&identifiers, vocabulary).fresh(&identifiers);
    Renamed {
        program: rewritten(program, &identifiers, &fresh),
        originals: fresh.into_iter().map(|(old, new)| (new, old)).collect(),
    }
}

pub fn renamed_copies(program: &Program, vocabulary: &Vocabulary, count: usize) -> Vec<Program> {
    let identifiers = identifiers(program, vocabulary);
    let mut naming = Naming::new(&identifiers, vocabulary);
    (0..count)
        .map(|_| rewritten(program, &identifiers, &naming.fresh(&identifiers)))
        .collect()
}

impl Naming {
    fn new(identifiers: &Identifiers, vocabulary: &Vocabulary) -> Naming {
        let reserved = vocabulary_names(vocabulary);
        let taken = identifiers.seen.union(&reserved).cloned().collect();
        Naming {
            reserved,
            taken,
            counter: 0,
        }
    }

    fn fresh(&mut self, identifiers: &Identifiers) -> BTreeMap<String, String> {
        let mut fresh = BTreeMap::new();
        for name in identifiers
            .declared
            .iter()
            .filter(|name| !self.reserved.contains(*name))
        {
            let lead: String = name.chars().take(1).collect();
            let replacement = loop {
                let candidate = format!("{lead}ev{}", self.counter);
                self.counter += 1;
                if !self.taken.contains(&candidate) {
                    break candidate;
                }
            };
            self.taken.insert(replacement.clone());
            fresh.insert(name.clone(), replacement);
        }
        fresh
    }
}

fn rewritten(
    program: &Program,
    identifiers: &Identifiers,
    fresh: &BTreeMap<String, String>,
) -> Program {
    let sources = program
        .sources
        .iter()
        .zip(&identifiers.occurrences)
        .map(|(source, occurrences)| ProgramSource {
            path: source.path.clone(),
            text: replaced(&source.text, occurrences, fresh),
        })
        .collect();
    program.with_sources(sources)
}

pub fn restored(text: &str, originals: &BTreeMap<String, String>) -> String {
    let mut result = String::new();
    let mut word = String::new();
    for character in text.chars().chain(['\0']) {
        if character.is_alphanumeric() || character == '_' {
            word.push(character);
            continue;
        }
        result.push_str(originals.get(&word).unwrap_or(&word));
        word.clear();
        if character != '\0' {
            result.push(character);
        }
    }
    result
}

fn identifiers(program: &Program, vocabulary: &Vocabulary) -> Identifiers {
    let mut found = Identifiers {
        occurrences: Vec::new(),
        declared: BTreeSet::new(),
        seen: BTreeSet::new(),
    };
    for source in &program.sources {
        let parsed = parse(
            &Source {
                path: source.path.clone(),
                text: source.text.clone(),
            },
            vocabulary,
        );
        let mut occurrences = Vec::new();
        for event in parsed.root().preorder() {
            let Walk::Enter(ElementView::Token(token)) = event else {
                continue;
            };
            if token.kind() != SyntaxKind::Ident {
                continue;
            }
            let text: String = token.text().nfc().collect();
            let declares = token
                .parent()
                .is_some_and(|parent| DECLARATIONS.contains(&parent.kind()));
            if declares {
                found.declared.insert(text.clone());
            }
            found.seen.insert(text.clone());
            let range = token.range();
            if let (Ok(start), Ok(end)) = (usize::try_from(range.start), usize::try_from(range.end))
            {
                occurrences.push((start, end, text));
            }
        }
        found.occurrences.push(occurrences);
    }
    found
}

fn replaced(
    text: &str,
    occurrences: &[(usize, usize, String)],
    fresh: &BTreeMap<String, String>,
) -> String {
    let mut result = String::new();
    let mut copied = 0;
    for (start, end, name) in occurrences {
        let Some(replacement) = fresh.get(name) else {
            continue;
        };
        let Some(before) = text.get(copied..*start) else {
            continue;
        };
        result.push_str(before);
        result.push_str(replacement);
        copied = *end;
    }
    result.push_str(text.get(copied..).unwrap_or_default());
    result
}

fn vocabulary_names(vocabulary: &Vocabulary) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = BUILTINS
        .iter()
        .chain(KEYWORDS)
        .chain(RESERVED_WORDS)
        .map(|name| (*name).to_string())
        .collect();
    for kind in vocabulary.node_kinds() {
        names.insert(kind.name.clone());
        names.extend(kind.attrs.iter().map(|attr| attr.name.clone()));
    }
    for kind in vocabulary.wire_kinds() {
        names.insert(kind.name.clone());
        names.extend(kind.attrs.iter().map(|attr| attr.name.clone()));
    }
    for decl in vocabulary.enums() {
        names.insert(decl.name.clone());
        names.extend(decl.variants.iter().cloned());
    }
    for family in vocabulary.families() {
        names.insert(family.name.clone());
        names.extend(family.units.iter().map(|unit| unit.name.clone()));
    }
    names
}
