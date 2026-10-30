#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Dimensions {
    Known(usize),
    Unknown,
}

impl Dimensions {
    pub fn holds_array(self) -> bool {
        self != Dimensions::Known(0)
    }

    pub fn agreed(dimensions: impl IntoIterator<Item = Dimensions>) -> Dimensions {
        let mut agreed: Option<Dimensions> = None;
        for found in dimensions {
            agreed = Some(match agreed {
                None => found,
                Some(known) if known == found => known,
                Some(known) if known.holds_array() || found.holds_array() => Dimensions::Unknown,
                Some(known) => known,
            });
        }
        agreed.unwrap_or(Dimensions::Known(0))
    }
}
