use std::ops::Range;

use symphonyscript_core::Vocabulary;

use super::fault_catalogue::BracketKind;
use crate::source::Source;
use crate::syntax::{SyntaxKind, lex};

const ITEM_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::InputKw,
    SyntaxKind::LetKw,
    SyntaxKind::FnKw,
    SyntaxKind::PatternKw,
];

const FLAGS: &[SyntaxKind] = &[
    SyntaxKind::Quote,
    SyntaxKind::Bang,
    SyntaxKind::Question,
    SyntaxKind::Star,
    SyntaxKind::Star2,
    SyntaxKind::Plus,
    SyntaxKind::Tilde,
    SyntaxKind::At,
    SyntaxKind::Dollar,
];

#[derive(Clone, Debug)]
pub(super) struct LineShape {
    pub(super) first_token: Range<u32>,
    first: SyntaxKind,
    first_is_kind: bool,
    second: Option<SyntaxKind>,
    second_adjacent: bool,
    stray_colon: bool,
}

impl LineShape {
    pub(super) fn of(text: &str, vocabulary: &Vocabulary) -> LineShape {
        let source = Source {
            path: "line.sym".into(),
            text: text.into(),
        };
        let line_end = text.find('\n').unwrap_or(text.len()) as u32;
        let tokens: Vec<_> = lex(&source, vocabulary)
            .tokens
            .into_iter()
            .filter(|token| token.range.start < line_end)
            .filter(|token| {
                !token.kind.is_trivia()
                    && !matches!(
                        token.kind,
                        SyntaxKind::Newline | SyntaxKind::Indent | SyntaxKind::Dedent
                    )
            })
            .collect();
        let first = tokens.first().expect("an item's first line holds a token");
        let name = text
            .get(first.range.start as usize..first.range.end as usize)
            .unwrap_or_default();
        let second = tokens.get(1);
        let mut depth = 0usize;
        let mut header = false;
        let mut stray_colon = false;
        for token in &tokens {
            match token.kind {
                SyntaxKind::LParen | SyntaxKind::LBracket => depth += 1,
                SyntaxKind::RParen | SyntaxKind::RBracket => depth = depth.saturating_sub(1),
                SyntaxKind::ForKw if depth == 0 => header = true,
                SyntaxKind::Colon if depth == 0 && header => header = false,
                SyntaxKind::Colon if depth == 0 => stray_colon = true,
                _ => {}
            }
        }
        LineShape {
            first_token: first.range.clone(),
            first: first.kind,
            first_is_kind: vocabulary.node_kind_named(name).is_some()
                || vocabulary.wire_kind_named(name).is_some(),
            second: second.map(|token| token.kind),
            second_adjacent: second.is_some_and(|token| token.range.start == first.range.end),
            stray_colon,
        }
    }

    pub(super) fn ends_unclosed(&self, bracket: BracketKind) -> bool {
        if ITEM_KEYWORDS.contains(&self.first) {
            return true;
        }
        match bracket {
            BracketKind::Expression => {
                self.first == SyntaxKind::Ident
                    && matches!(self.second, Some(SyntaxKind::Ident | SyntaxKind::AsKw))
            }
            BracketKind::Bundle => self.stray_colon,
            BracketKind::Other => false,
        }
    }

    pub(super) fn resynchronizes(&self, column_zero: bool) -> bool {
        if ITEM_KEYWORDS.contains(&self.first)
            || matches!(
                self.first,
                SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::ForKw | SyntaxKind::WhenKw
            )
        {
            return true;
        }
        if self.first != SyntaxKind::Ident {
            return false;
        }
        let flag = self.second_adjacent && self.second.is_some_and(|kind| FLAGS.contains(&kind));
        if column_zero {
            return flag
                || matches!(
                    self.second,
                    None | Some(SyntaxKind::Ident | SyntaxKind::AsKw)
                );
        }
        flag || matches!(
            self.second,
            Some(
                SyntaxKind::Ident
                    | SyntaxKind::AsKw
                    | SyntaxKind::Pipe
                    | SyntaxKind::WhenKw
                    | SyntaxKind::Colon
            )
        ) || (self.second == Some(SyntaxKind::Slash) && self.first_is_kind)
    }
}
