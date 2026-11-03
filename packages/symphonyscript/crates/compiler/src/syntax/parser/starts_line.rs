use symphonyscript_core::Vocabulary;

use super::placements::FLAG_TOKENS;
use crate::syntax::lexer::LineToken;
use crate::syntax::lexer::{LineStart, LineTokens};
use crate::syntax::nfc::nfc;
use crate::syntax::syntax_kind::SyntaxKind;

pub(super) fn starts_line(line: &LineStart<'_>, text: &str, vocabulary: &Vocabulary) -> bool {
    match line.first.kind {
        SyntaxKind::LetKw
        | SyntaxKind::StateKw
        | SyntaxKind::StreamKw
        | SyntaxKind::ChannelKw
        | SyntaxKind::FnKw
        | SyntaxKind::PatternKw
        | SyntaxKind::LParen
        | SyntaxKind::LBracket
        | SyntaxKind::ForKw
        | SyntaxKind::WhenKw => true,
        SyntaxKind::Ident => name_starts_line(line, text, vocabulary),
        _ => false,
    }
}

fn name_starts_line(line: &LineStart<'_>, text: &str, vocabulary: &Vocabulary) -> bool {
    let mut rest = significant(line.rest);
    let Some(next) = rest.next() else {
        return line.top_level;
    };
    let adjacent = next.start == line.first.end;
    let flag = adjacent && FLAG_TOKENS.contains(&next.kind);
    if line.top_level {
        return flag || matches!(next.kind, SyntaxKind::Ident | SyntaxKind::AsKw);
    }
    if next.kind == SyntaxKind::Slash {
        return is_kind(&line.first, text, vocabulary)
            || (adjacent && !line.closes_region && default_value_ends_line(rest));
    }
    flag || matches!(
        next.kind,
        SyntaxKind::Ident
            | SyntaxKind::AsKw
            | SyntaxKind::Pipe
            | SyntaxKind::WhenKw
            | SyntaxKind::Colon
    )
}

fn significant<'t>(tokens: LineTokens<'t>) -> impl Iterator<Item = LineToken> + 't {
    tokens.iter().filter(|token| !token.kind.is_trivia())
}

fn is_kind(token: &LineToken, text: &str, vocabulary: &Vocabulary) -> bool {
    let Some(name) = text.get(token.start as usize..token.end as usize) else {
        return false;
    };
    let name = nfc(name);
    vocabulary.node_kind_named(&name).is_some() || vocabulary.wire_kind_named(&name).is_some()
}

fn default_value_ends_line(mut rest: impl Iterator<Item = LineToken>) -> bool {
    let Some(first) = rest.next() else {
        return false;
    };
    let after = match first.kind {
        SyntaxKind::LParen => skip_closed(&mut rest, SyntaxKind::LParen, SyntaxKind::RParen),
        SyntaxKind::StringStart => {
            skip_closed(&mut rest, SyntaxKind::StringStart, SyntaxKind::StringEnd)
        }
        SyntaxKind::Minus => rest
            .next()
            .filter(|token| token.kind == SyntaxKind::Number)
            .map(|_| rest.next()),
        SyntaxKind::Number | SyntaxKind::TrueKw | SyntaxKind::FalseKw => Some(rest.next()),
        SyntaxKind::Ident => skip_name_value(&mut rest),
        SyntaxKind::ToKw => skip_to_value(&mut rest),
        _ => None,
    };
    after.is_some_and(|after| {
        after.is_none_or(|token| {
            matches!(
                token.kind,
                SyntaxKind::Pipe | SyntaxKind::WhenKw | SyntaxKind::Colon
            )
        })
    })
}

fn skip_closed(
    rest: &mut impl Iterator<Item = LineToken>,
    open: SyntaxKind,
    close: SyntaxKind,
) -> Option<Option<LineToken>> {
    let mut depth = 1usize;
    while let Some(token) = rest.next() {
        if token.kind == open {
            depth += 1;
        } else if token.kind == close {
            depth -= 1;
            if depth == 0 {
                return Some(rest.next());
            }
        }
    }
    None
}

fn skip_name_value(rest: &mut impl Iterator<Item = LineToken>) -> Option<Option<LineToken>> {
    let mut dotted = false;
    loop {
        let Some(separator) = rest.next() else {
            return Some(None);
        };
        match separator.kind {
            SyntaxKind::Dot => dotted = true,
            SyntaxKind::Colon2 if !dotted => {
                return (rest.next()?.kind == SyntaxKind::Ident).then(|| rest.next());
            }
            _ => return Some(Some(separator)),
        }
        if rest.next()?.kind != SyntaxKind::Ident {
            return None;
        }
    }
}

fn skip_to_value(rest: &mut impl Iterator<Item = LineToken>) -> Option<Option<LineToken>> {
    if rest.next()?.kind != SyntaxKind::Dot || rest.next()?.kind != SyntaxKind::Ident {
        return None;
    }
    loop {
        let Some(separator) = rest.next() else {
            return Some(None);
        };
        if separator.kind != SyntaxKind::Dot {
            return Some(Some(separator));
        }
        if rest.next()?.kind != SyntaxKind::Ident {
            return None;
        }
    }
}
