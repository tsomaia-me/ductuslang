//! The `MemberDriver` enum: the closed set of membership-driver tags.

/// What makes a member of a `members` set present (IR_GRAMMAR §3's
/// `member_driver`, §15.4.6).
///
/// ```text
/// member_driver ::= 'permanent' | 'keyed_template' | 'gate_guarded'
/// ```
///
/// The set is CLOSED at exactly these three (033-81), and all three are
/// snake_case words in the NAME charset — no driver tag contains a hyphen
/// (R39), which is what keeps them spellable as ordinary words.
///
/// A vocabulary scalar, so `PartialEq` is not the equality R17 withholds — the
/// `Prim` reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberDriver {
    /// `permanent` — a static position, always a member.
    Permanent,
    /// `keyed_template` — a `repeat`-driven position, present once per key.
    KeyedTemplate,
    /// `gate_guarded` — a gated-arm position, present iff its gate is
    /// effectively active.
    GateGuarded,
}

impl MemberDriver {
    /// The one spelling of this driver in `.duir` text (§3's `member_driver`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberDriver::Permanent => "permanent",
            MemberDriver::KeyedTemplate => "keyed_template",
            MemberDriver::GateGuarded => "gate_guarded",
        }
    }

    /// The driver `word` names, or `None` when it names none.
    ///
    /// The soft-keyword classifier for the right-hand side of a `member` pair
    /// (R46). It is also what closes the pair when the left-hand `PATH` could
    /// still be growing: after a `member`'s colon an INT continues the path as
    /// an ordinal, and a word that names a driver ends it (§3's boundary note).
    #[must_use]
    pub fn from_word(word: &str) -> Option<MemberDriver> {
        let driver = match word {
            "permanent" => MemberDriver::Permanent,
            "keyed_template" => MemberDriver::KeyedTemplate,
            "gate_guarded" => MemberDriver::GateGuarded,
            _ => return None,
        };

        Some(driver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: [MemberDriver; 3] = [
        MemberDriver::Permanent,
        MemberDriver::KeyedTemplate,
        MemberDriver::GateGuarded,
    ];

    #[test]
    fn every_driver_spells_its_grammar_word() {
        assert_eq!(MemberDriver::Permanent.as_str(), "permanent");
        assert_eq!(MemberDriver::KeyedTemplate.as_str(), "keyed_template");
        assert_eq!(MemberDriver::GateGuarded.as_str(), "gate_guarded");
    }

    #[test]
    fn from_word_inverts_as_str_for_every_driver() {
        let mut words = HashSet::new();

        for driver in ALL {
            assert_eq!(MemberDriver::from_word(driver.as_str()), Some(driver));
            assert!(words.insert(driver.as_str()), "{} twice", driver.as_str());
        }

        assert_eq!(words.len(), 3);
    }

    // The hyphenated spellings are what R39 respelled away; they are not
    // drivers, and nothing accepts them as aliases.
    #[test]
    fn from_word_rejects_the_hyphenated_spellings_and_everything_else() {
        for word in [
            "keyed-template",
            "gate-guarded",
            "keyed",
            "template",
            "Permanent",
            "permanent ",
            "",
            "banana",
        ] {
            assert_eq!(MemberDriver::from_word(word), None, "{word}");
        }
    }
}
