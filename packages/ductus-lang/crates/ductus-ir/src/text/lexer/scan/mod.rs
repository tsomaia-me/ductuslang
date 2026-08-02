//! The pure recognizers — one function per lexeme family, one file per
//! recognizer (R14, S1).
//!
//! Each takes the REMAINING input slice and answers two questions: does my
//! lexeme start here, and if so how many bytes long is it and what does it
//! mean. No cursor, no source, no state. A recognizer test is one call and one
//! assertion, and a later construct increment adds a recognizer file plus a
//! dispatch arm without reopening the driver.
//!
//! Recognizers that can fail mid-lexeme answer `Option<Result<…>>`: the `None`
//! says "not my lexeme, try the next recognizer", the `Err` says "mine, and
//! malformed". Their error offsets are relative to the slice; the driver adds
//! the cursor (R14 — the driver owns all span bookkeeping).
//!
//! `chars` and `payload` are folder-private plumbing shared by sibling
//! recognizers — S1's private exemption. They are re-exported nowhere, and
//! nothing outside this folder can name them.
//!
//! The lexeme rules are IR_GRAMMAR §2 (§15.4.6).

mod chars;
mod payload;

#[cfg(test)]
mod test_support;

pub(crate) mod bid;
pub(crate) mod ident;
pub(crate) mod number;
pub(crate) mod punctuation;
pub(crate) mod quoted_key;
pub(crate) mod string;

pub(crate) use self::bid::bid;
pub(crate) use self::ident::ident;
pub(crate) use self::number::number;
pub(crate) use self::punctuation::punctuation;
pub(crate) use self::quoted_key::quoted_key;
pub(crate) use self::string::string;
