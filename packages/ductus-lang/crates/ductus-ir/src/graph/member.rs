//! The `Member` struct: one edge of a `members` set.

use super::member_driver::MemberDriver;
use crate::path::CellPath;

/// One member edge — IR_GRAMMAR §3's `member` (§15.4.6).
///
/// ```text
/// member_set ::= '[' (member (',' member)*)? ']'
/// member     ::= PATH ':' member_driver
/// ```
///
/// A `members` set lists its edges in MEMBER ORDER — walk order for a
/// membership-descriptor cell — so the position of a `Member` in the vector is
/// meaning, not incidental (033-81). Each edge names the member cell and how
/// its membership is driven.
///
/// The pair separator is `:` and a `PATH` segment may itself end with a `:N`
/// ordinal, which is the one ambiguity here. The disambiguator is the grammar's
/// own: after a colon an INT continues the path as an ordinal, and a word that
/// names a driver closes the pair (§3's boundary note).
///
/// No span (R17): spans ride the named ENTRIES — the cell this set belongs
/// to — and a load-time diagnostic about a member points at that cell.
#[derive(Debug, Clone)]
pub struct Member {
    /// The member cell's path.
    pub path: CellPath,
    /// What makes it present.
    pub driver: MemberDriver,
}
