pub(crate) fn is_ident_start(c: char) -> bool {
    c == '_' || c == '#' || unicode_ident::is_xid_start(c)
}

pub(crate) fn is_ident_continue(c: char) -> bool {
    c == '#' || unicode_ident::is_xid_continue(c)
}

pub(super) fn is_unexpected(c: char, crlf: bool) -> bool {
    !(matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '"'
            | '{'
            | '}'
            | '('
            | ')'
            | '['
            | ']'
            | ','
            | ':'
            | '='
            | '|'
            | '/'
            | '.'
            | '+'
            | '-'
            | '*'
            | '%'
            | '<'
            | '>'
            | '!'
            | '\''
            | '?'
            | '~'
            | '@'
            | '$'
            | '^'
            | ';'
    ) || (c == '\r' && crlf)
        || c.is_ascii_digit()
        || is_ident_start(c))
}
