use super::bound_form::BoundForm;
use super::piecewise::Piecewise;

pub(crate) fn slice_transform(lower: BoundForm, upper: BoundForm) -> Piecewise {
    let length = Piecewise::affine(0, 1);
    let zero = Piecewise::constant(0);
    let clamped = |bound: Piecewise| bound.max(&zero).min(&length);
    let position = |form: BoundForm, absent: &Piecewise| match form {
        BoundForm::Absent | BoundForm::Reactive => absent.clone(),
        BoundForm::Start(value) => clamped(Piecewise::constant(value)),
        BoundForm::End(offset) => clamped(Piecewise::affine(offset.saturating_neg(), 1)),
    };
    let start = position(lower, &zero);
    let end = position(upper, &length);
    end.add(&start.neg()).max(&zero).prefix_max()
}
