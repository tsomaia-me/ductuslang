use super::bound_form::BoundForm;
use super::piecewise::Piecewise;

pub(crate) const LIMIT: i128 = 1 << 31;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Extent {
    Fixed(i128),
    Variable(Piecewise),
}

impl Extent {
    pub fn open(length: u32) -> Extent {
        if length == super::folds::ANY_LENGTH {
            Extent::Variable(Piecewise::constant(0))
        } else {
            Extent::Fixed(i128::from(length))
        }
    }

    pub fn capacity(&self) -> Piecewise {
        match self {
            Extent::Fixed(length) => Piecewise::constant(*length),
            Extent::Variable(capacity) => capacity.clone(),
        }
    }

    pub fn held(&self) -> i128 {
        match self {
            Extent::Fixed(length) => *length,
            Extent::Variable(capacity) => capacity.start().floor(),
        }
    }

    pub fn join(&self, other: &Extent) -> Extent {
        match (self, other) {
            (Extent::Fixed(a), Extent::Fixed(b)) => Extent::Fixed(a.saturating_add(*b)),
            _ => Extent::Variable(self.capacity().add(&other.capacity())),
        }
    }

    pub fn choice(&self, other: &Extent) -> Extent {
        match (self, other) {
            (Extent::Fixed(a), Extent::Fixed(b)) if a == b => Extent::Fixed(*a),
            _ => Extent::Variable(self.capacity().max(&other.capacity())),
        }
    }

    pub fn reaches_limit(&self) -> bool {
        match self {
            Extent::Fixed(length) => *length >= LIMIT,
            Extent::Variable(capacity) => capacity.start().floor() >= LIMIT,
        }
    }
}

pub(crate) fn fixed_slice(lower: BoundForm, upper: BoundForm, base: &Extent) -> Option<i128> {
    let Extent::Fixed(length) = base else {
        return None;
    };
    let position = |form: BoundForm, absent: i128| match form {
        BoundForm::Absent => Some(absent),
        BoundForm::Reactive(_) => None,
        BoundForm::Start(value) => Some(value.clamp(0, *length)),
        BoundForm::End(offset) => Some(length.saturating_sub(offset).clamp(0, *length)),
    };
    let start = position(lower, 0)?;
    let end = position(upper, *length)?;
    Some(end.max(start) - start)
}

pub(crate) fn outside_i64(form: BoundForm) -> bool {
    match form {
        BoundForm::Start(value) | BoundForm::End(value) => i64::try_from(value).is_err(),
        BoundForm::Absent | BoundForm::Reactive(_) => false,
    }
}
