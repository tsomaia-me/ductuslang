use super::bound_form::BoundForm;
use super::extreme::Extreme;
use super::piecewise::Piecewise;

pub(crate) fn slice_transform(lower: BoundForm, upper: BoundForm) -> Piecewise {
    let length = Piecewise::affine(0, 1);
    let zero = Piecewise::constant(0);
    let clamped = |bound: Piecewise| bound.max(&zero).min(&length);
    let start_at = |value: i128| clamped(Piecewise::constant(value));
    let end_at = |offset: i128| clamped(Piecewise::affine(offset.saturating_neg(), 1));
    let position = |form: BoundForm, absent: &Piecewise| match form {
        BoundForm::Absent | BoundForm::Reactive(Extreme::Unbounded) => absent.clone(),
        BoundForm::Start(value) | BoundForm::Reactive(Extreme::Start(value)) => start_at(value),
        BoundForm::End(offset) | BoundForm::Reactive(Extreme::End(offset)) => end_at(offset),
    };
    let start = position(lower, &zero);
    let end = position(upper, &length);
    end.add(&start.neg()).max(&zero).prefix_max()
}
