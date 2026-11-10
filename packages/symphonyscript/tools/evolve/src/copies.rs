use std::collections::{BTreeMap, BTreeSet};

use symphonyscript::Source;
use symphonyscript::syntax::{ElementView, NodeView, SyntaxKind, TokenView, Walk, parse};
use symphonyscript_core::{BUILTINS, KEYWORDS, RESERVED_WORDS, Vocabulary};
use unicode_normalization::UnicodeNormalization;

use crate::program::{Program, ProgramSource};
use crate::renamed::{DIGITS, LOWER, UPPER, is_identifier, vocabulary_names};

const TOP_LEVEL_DECLARATIONS: &[SyntaxKind] = &[
    SyntaxKind::CellDecl,
    SyntaxKind::LetDecl,
    SyntaxKind::FnDecl,
    SyntaxKind::PatternDecl,
    SyntaxKind::TypeDecl,
];

const SEARCHED_NEIGHBOURS: u32 = 512;

#[derive(Clone, PartialEq, Eq, Debug)]
enum Role {
    Plain,
    Member,
    Key(Option<String>),
    Foreign,
}

struct Occurrence {
    start: usize,
    end: usize,
    name: String,
    role: Role,
}

struct Names {
    occurrences: Vec<Vec<Occurrence>>,
    top_level: BTreeSet<String>,
    program_members: BTreeSet<String>,
    vocabulary_members: BTreeSet<String>,
    kept: BTreeSet<String>,
    taken: BTreeSet<String>,
}

struct Naming {
    taken: BTreeSet<String>,
    counters: BTreeMap<String, u32>,
    ascii_counters: BTreeMap<(bool, usize), usize>,
    vocabulary: Vocabulary,
}

pub fn renamed_copies(program: &Program, vocabulary: &Vocabulary, count: usize) -> Vec<Program> {
    let names = names(program, vocabulary);
    let mut naming = Naming {
        taken: names.taken.clone(),
        counters: BTreeMap::new(),
        ascii_counters: BTreeMap::new(),
        vocabulary: vocabulary.clone(),
    };
    (0..count)
        .map_while(|_| {
            naming
                .fresh(&names)
                .map(|fresh| rewritten(program, &names, &fresh))
        })
        .collect()
}

impl Naming {
    fn fresh(&mut self, names: &Names) -> Option<BTreeMap<String, String>> {
        let mut fresh = BTreeMap::new();
        for name in names
            .top_level
            .iter()
            .filter(|name| !names.kept.contains(*name) && is_identifier(name))
        {
            let replacement = if name.is_ascii() {
                self.ascii_replacement(name)?
            } else {
                self.wide_replacement(name)?
            };
            self.taken.insert(replacement.clone());
            fresh.insert(name.clone(), replacement);
        }
        Some(fresh)
    }

    fn ascii_replacement(&mut self, name: &str) -> Option<String> {
        let width = name.len();
        let upper = name.starts_with(|character: char| character.is_ascii_uppercase());
        let counter = self.ascii_counters.entry((upper, width)).or_insert(0);
        let letters = first_letters(upper);
        let tail = u32::try_from(width.checked_sub(1)?).ok()?;
        let limit = letters
            .len()
            .saturating_mul(DIGITS.len().saturating_pow(tail));
        while *counter < limit {
            let candidate = candidate(&letters, *counter, width);
            *counter += 1;
            if !self.taken.contains(&candidate)
                && (candidate.is_ascii() || lexes_as_name(&candidate, &self.vocabulary))
            {
                return Some(candidate);
            }
        }
        None
    }

    fn wide_replacement(&mut self, name: &str) -> Option<String> {
        let scalars: Vec<char> = name.chars().collect();
        let position = scalars
            .iter()
            .position(char::is_ascii_alphanumeric)
            .unwrap_or(0);
        let original = scalars[position];
        let counter = self.counters.entry(name.to_string()).or_insert(0);
        while *counter < SEARCHED_NEIGHBOURS {
            *counter += 1;
            let Some(replaced) = neighbour(original, *counter) else {
                continue;
            };
            let mut candidate = scalars.clone();
            candidate[position] = replaced;
            let candidate: String = candidate.into_iter().collect();
            if !self.taken.contains(&candidate) && lexes_as_name(&candidate, &self.vocabulary) {
                return Some(candidate);
            }
        }
        None
    }
}

fn neighbour(original: char, step: u32) -> Option<char> {
    if original.is_ascii_alphanumeric() {
        let alphabet: &[u8] = if original.is_ascii_digit() {
            b"0123456789"
        } else if original.is_ascii_uppercase() {
            UPPER
        } else {
            LOWER
        };
        let index = alphabet
            .iter()
            .position(|byte| char::from(*byte) == original)?;
        let next = (index + usize::try_from(step).ok()?) % alphabet.len();
        return (next != index).then(|| char::from(alphabet[next]));
    }
    let code = u32::from(original);
    let block = code & !0x7f;
    let shifted = block + ((code - block + step) & 0x7f);
    let replaced = char::from_u32(shifted)?;
    (replaced != original && replaced.is_alphabetic() == original.is_alphabetic())
        .then_some(replaced)
}

fn lexes_as_name(candidate: &str, vocabulary: &Vocabulary) -> bool {
    if candidate.nfc().collect::<String>() != candidate {
        return false;
    }
    let parsed = parse(
        &Source {
            path: "name.sym".to_string(),
            text: format!("let {candidate} = 1\n"),
        },
        vocabulary,
    );
    parsed.diagnostics.is_empty()
        && parsed.root().preorder().any(|event| {
            matches!(event, Walk::Enter(ElementView::Token(token))
                if token.kind() == SyntaxKind::Ident && token.text() == candidate)
        })
}

fn rewritten(program: &Program, names: &Names, fresh: &BTreeMap<String, String>) -> Program {
    let sources = program
        .sources
        .iter()
        .zip(&names.occurrences)
        .map(|(source, occurrences)| ProgramSource {
            path: source.path.clone(),
            text: replaced(&source.text, occurrences, names, fresh),
        })
        .collect();
    program.with_sources(sources)
}

fn replaced(
    text: &str,
    occurrences: &[Occurrence],
    names: &Names,
    fresh: &BTreeMap<String, String>,
) -> String {
    let mut result = String::new();
    let mut copied = 0;
    for occurrence in occurrences {
        let Some(replacement) = fresh.get(&occurrence.name) else {
            continue;
        };
        if !names.substitutes(occurrence, fresh) {
            continue;
        }
        let Some(before) = text.get(copied..occurrence.start) else {
            continue;
        };
        let spelled = text
            .get(occurrence.start..occurrence.end)
            .unwrap_or_default();
        result.push_str(before);
        result.push_str(&respelled(spelled, &occurrence.name, replacement));
        copied = occurrence.end;
    }
    result.push_str(text.get(copied..).unwrap_or_default());
    result
}

fn respelled(spelled: &str, name: &str, replacement: &str) -> String {
    if spelled == name || spelled.nfd().collect::<String>() != spelled {
        replacement.to_string()
    } else {
        replacement.nfd().collect()
    }
}

impl Names {
    fn substitutes(&self, occurrence: &Occurrence, fresh: &BTreeMap<String, String>) -> bool {
        match &occurrence.role {
            Role::Plain => true,
            Role::Foreign => false,
            Role::Key(Some(owner)) => fresh.contains_key(owner),
            Role::Member | Role::Key(None) => {
                !self.vocabulary_members.contains(&occurrence.name)
                    || self.program_members.contains(&occurrence.name)
            }
        }
    }
}

fn names(program: &Program, vocabulary: &Vocabulary) -> Names {
    let mut found = Names {
        occurrences: Vec::new(),
        top_level: BTreeSet::new(),
        program_members: BTreeSet::new(),
        vocabulary_members: BTreeSet::new(),
        kept: kept_names(vocabulary),
        taken: vocabulary_names(vocabulary),
    };
    found.taken.extend(
        vocabulary
            .natives()
            .iter()
            .map(|native| native.name.clone()),
    );
    for prelude in vocabulary.prelude() {
        let parsed = parse(
            &Source {
                path: prelude.path.clone(),
                text: prelude.text.clone(),
            },
            vocabulary,
        );
        let root = parsed.root();
        found.taken.extend(top_level_names(&root));
        for event in root.preorder() {
            if let Walk::Enter(ElementView::Token(token)) = event
                && token.kind() == SyntaxKind::Ident
            {
                found.taken.insert(token.text().nfc().collect());
                if token
                    .parent()
                    .is_some_and(|parent| parent.kind() == SyntaxKind::FieldDecl)
                {
                    found
                        .vocabulary_members
                        .insert(token.text().nfc().collect());
                }
            }
        }
    }
    for kind in vocabulary.node_kinds() {
        found
            .vocabulary_members
            .extend(kind.attrs.iter().map(|attr| attr.name.clone()));
    }
    for kind in vocabulary.wire_kinds() {
        found
            .vocabulary_members
            .extend(kind.attrs.iter().map(|attr| attr.name.clone()));
    }
    for source in &program.sources {
        let parsed = parse(
            &Source {
                path: source.path.clone(),
                text: source.text.clone(),
            },
            vocabulary,
        );
        let root = parsed.root();
        found.top_level.extend(top_level_names(&root));
        let mut occurrences = Vec::new();
        for event in root.preorder() {
            let Walk::Enter(ElementView::Token(token)) = event else {
                continue;
            };
            if token.kind() != SyntaxKind::Ident {
                continue;
            }
            let name: String = token.text().nfc().collect();
            let role = role(&token);
            if declares_member(&token) {
                found.program_members.insert(name.clone());
            }
            found.taken.insert(name.clone());
            let range = token.range();
            if let (Ok(start), Ok(end)) = (usize::try_from(range.start), usize::try_from(range.end))
            {
                occurrences.push(Occurrence {
                    start,
                    end,
                    name,
                    role,
                });
            }
        }
        found.occurrences.push(occurrences);
    }
    found
}

fn top_level_names(root: &NodeView<'_>) -> Vec<String> {
    let mut names = Vec::new();
    for item in root.children() {
        let idents = item
            .children_with_tokens()
            .filter_map(ElementView::into_token)
            .filter(|token| token.kind() == SyntaxKind::Ident)
            .map(|token| token.text().nfc().collect::<String>());
        if item.kind() == SyntaxKind::Placement {
            names.extend(idents.skip(1));
        } else if TOP_LEVEL_DECLARATIONS.contains(&item.kind()) {
            names.extend(idents.take(1));
        }
    }
    names
}

fn role(token: &TokenView<'_>) -> Role {
    let Some(parent) = token.parent() else {
        return Role::Plain;
    };
    if previous_significant(token).is_some_and(|previous| previous.kind() == SyntaxKind::Dot) {
        return Role::Member;
    }
    match parent.kind() {
        SyntaxKind::Field | SyntaxKind::FieldDecl | SyntaxKind::ToExpr => Role::Member,
        SyntaxKind::QualifiedName => Role::Foreign,
        SyntaxKind::AttrEntry if first_ident(&parent).as_ref() == Some(token) => {
            Role::Key(placement_kind(&parent))
        }
        SyntaxKind::NamedArg if first_ident(&parent).as_ref() == Some(token) => {
            Role::Key(callee(&parent))
        }
        _ => Role::Plain,
    }
}

fn declares_member(token: &TokenView<'_>) -> bool {
    let Some(parent) = token.parent() else {
        return false;
    };
    match parent.kind() {
        SyntaxKind::FieldDecl => first_ident(&parent).as_ref() == Some(token),
        SyntaxKind::Placement | SyntaxKind::WirePlacement => {
            previous_significant(token).is_some_and(|previous| previous.kind() == SyntaxKind::AsKw)
        }
        _ => false,
    }
}

fn previous_significant<'p>(token: &TokenView<'p>) -> Option<TokenView<'p>> {
    let mut previous = token.prev_token();
    while let Some(candidate) = previous {
        if !matches!(
            candidate.kind(),
            SyntaxKind::Whitespace | SyntaxKind::Comment
        ) {
            return Some(candidate);
        }
        previous = candidate.prev_token();
    }
    None
}

fn first_ident<'p>(node: &NodeView<'p>) -> Option<TokenView<'p>> {
    node.children_with_tokens()
        .filter_map(ElementView::into_token)
        .find(|token| token.kind() == SyntaxKind::Ident)
}

fn placement_kind(entry: &NodeView<'_>) -> Option<String> {
    let placement = entry.ancestors().find(|node| {
        matches!(
            node.kind(),
            SyntaxKind::Placement | SyntaxKind::WirePlacement
        )
    })?;
    first_ident(&placement).map(|token| token.text().nfc().collect())
}

fn callee(argument: &NodeView<'_>) -> Option<String> {
    let call = argument
        .ancestors()
        .find(|node| node.kind() == SyntaxKind::CallExpr)?;
    let target = call.children().next()?;
    let token = match target.kind() {
        SyntaxKind::NameRef => first_ident(&target),
        SyntaxKind::FieldExpr => target
            .children_with_tokens()
            .filter_map(ElementView::into_token)
            .filter(|token| token.kind() == SyntaxKind::Ident)
            .last(),
        _ => None,
    }?;
    Some(token.text().nfc().collect())
}

fn kept_names(vocabulary: &Vocabulary) -> BTreeSet<String> {
    let mut kept: BTreeSet<String> = BUILTINS
        .iter()
        .chain(KEYWORDS)
        .chain(RESERVED_WORDS)
        .map(|name| (*name).to_string())
        .collect();
    kept.extend(vocabulary.node_kinds().iter().map(|kind| kind.name.clone()));
    kept.extend(vocabulary.wire_kinds().iter().map(|kind| kind.name.clone()));
    for decl in vocabulary.enums() {
        kept.insert(decl.name.clone());
        kept.extend(decl.variants.iter().cloned());
    }
    kept
}

fn first_letters(upper: bool) -> Vec<char> {
    let ascii = if upper { UPPER } else { LOWER };
    let extra = ('\u{c0}'..='\u{24f}')
        .chain('\u{391}'..='\u{3c9}')
        .chain('\u{400}'..='\u{45f}')
        .filter(|letter| {
            letter.is_alphabetic()
                && letter.is_uppercase() == upper
                && letter.is_lowercase() != upper
                && letter.to_string().nfc().eq([*letter])
        });
    ascii
        .iter()
        .map(|byte| char::from(*byte))
        .chain(extra)
        .collect()
}

fn candidate(letters: &[char], index: usize, width: usize) -> String {
    let tail = u32::try_from(width.saturating_sub(1)).unwrap_or(u32::MAX);
    let ascii = LOWER
        .len()
        .saturating_mul(DIGITS.len().saturating_pow(tail));
    let (first_letters, mut rest) = if index < ascii {
        (&letters[..LOWER.len()], index)
    } else {
        (&letters[LOWER.len()..], index - ascii)
    };
    let mut result = vec![first_letters[rest % first_letters.len()]];
    rest /= first_letters.len();
    for _ in 1..width {
        result.push(char::from(DIGITS[rest % DIGITS.len()]));
        rest /= DIGITS.len();
    }
    result.into_iter().collect()
}
