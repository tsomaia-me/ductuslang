const TWO_TO_THE_127: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0;

pub(crate) fn represents(integer: i128, float: f64) -> bool {
    float.is_finite() && float.abs() < TWO_TO_THE_127 && float as i128 == integer
}
