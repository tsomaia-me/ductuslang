use super::ratio::Ratio;

type Segment = ((Ratio, Ratio), Option<(Ratio, Ratio)>);

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Piecewise {
    points: Vec<(Ratio, Ratio)>,
    tail: Ratio,
}

impl Piecewise {
    pub fn constant(value: i128) -> Piecewise {
        Piecewise {
            points: vec![(Ratio::zero(), Ratio::int(value))],
            tail: Ratio::zero(),
        }
    }

    pub fn affine(start: i128, slope: i128) -> Piecewise {
        Piecewise {
            points: vec![(Ratio::zero(), Ratio::int(start))],
            tail: Ratio::int(slope),
        }
    }

    pub fn at(&self, t: Ratio) -> Ratio {
        let mut previous: Option<(Ratio, Ratio)> = None;
        for &(point, value) in &self.points {
            if point == t {
                return value;
            }
            if point > t {
                return match previous {
                    Some((left, left_value)) => interpolate((left, left_value), (point, value), t),
                    None => value,
                };
            }
            previous = Some((point, value));
        }
        match previous {
            Some((last, value)) => value + self.tail * (t - last),
            None => Ratio::zero(),
        }
    }

    pub fn start(&self) -> Ratio {
        self.points
            .first()
            .map_or(Ratio::zero(), |(_, value)| *value)
    }

    fn breakpoints(&self) -> impl Iterator<Item = Ratio> + '_ {
        self.points.iter().map(|(point, _)| *point)
    }

    fn last_point(&self) -> Ratio {
        self.points
            .last()
            .map_or(Ratio::zero(), |(point, _)| *point)
    }

    fn from_points(points: Vec<Ratio>, value: impl Fn(Ratio) -> Ratio, tail: Ratio) -> Piecewise {
        let mut points: Vec<Ratio> = points;
        points.sort();
        points.dedup();
        let points = points
            .into_iter()
            .map(|point| (point, value(point)))
            .collect();
        Piecewise { points, tail }.simplified()
    }

    fn simplified(mut self) -> Piecewise {
        let mut kept: Vec<(Ratio, Ratio)> = Vec::with_capacity(self.points.len());
        for (index, &(point, value)) in self.points.iter().enumerate() {
            if !kept.is_empty() {
                let next_slope = match self.points.get(index + 1) {
                    Some(&(next, next_value)) => (next_value - value).div(next - point),
                    None => Some(self.tail),
                };
                if let (Some(&(previous, previous_value)), Some(next_slope)) =
                    (kept.last(), next_slope)
                    && (value - previous_value).div(point - previous) == Some(next_slope)
                {
                    continue;
                }
            }
            kept.push((point, value));
        }
        self.points = kept;
        self
    }

    pub fn add(&self, other: &Piecewise) -> Piecewise {
        let points: Vec<Ratio> = self.breakpoints().chain(other.breakpoints()).collect();
        Piecewise::from_points(points, |t| self.at(t) + other.at(t), self.tail + other.tail)
    }

    pub fn neg(&self) -> Piecewise {
        Piecewise {
            points: self
                .points
                .iter()
                .map(|(point, value)| (*point, -*value))
                .collect(),
            tail: -self.tail,
        }
    }

    pub fn max(&self, other: &Piecewise) -> Piecewise {
        let mut points: Vec<Ratio> = self.breakpoints().chain(other.breakpoints()).collect();
        points.sort();
        points.dedup();
        let difference = |t: Ratio| self.at(t) - other.at(t);
        let mut crossings = Vec::new();
        for pair in points.windows(2) {
            let &[left, right] = pair else {
                continue;
            };
            let (a, b) = (difference(left), difference(right));
            if ((a.is_negative() && b > Ratio::zero()) || (a > Ratio::zero() && b.is_negative()))
                && let Some(fraction) = a.div(a - b)
            {
                crossings.push(left + (right - left) * fraction);
            }
        }
        let last = points.last().copied().unwrap_or(Ratio::zero());
        let at_last = difference(last);
        let tail_difference = self.tail - other.tail;
        if !tail_difference.is_zero()
            && ((at_last.is_negative() && tail_difference > Ratio::zero())
                || (at_last > Ratio::zero() && tail_difference.is_negative()))
            && let Some(step) = (-at_last).div(tail_difference)
        {
            crossings.push(last + step);
        }
        points.extend(crossings);
        points.sort();
        points.dedup();
        let last = points.last().copied().unwrap_or(Ratio::zero());
        let (here, there) = (self.at(last), other.at(last));
        let tail = if here > there {
            self.tail
        } else if there > here {
            other.tail
        } else {
            self.tail.max(other.tail)
        };
        Piecewise::from_points(points, |t| self.at(t).max(other.at(t)), tail)
    }

    pub fn min(&self, other: &Piecewise) -> Piecewise {
        self.neg().max(&other.neg()).neg()
    }

    pub fn prefix_max(&self) -> Piecewise {
        let mut points: Vec<(Ratio, Ratio)> = Vec::new();
        let mut best: Option<Ratio> = None;
        let segments: Vec<Segment> = self
            .points
            .iter()
            .enumerate()
            .map(|(index, start)| (*start, self.points.get(index + 1).copied()))
            .collect();
        for ((left, left_value), next) in segments {
            let current = best.map_or(left_value, |best| best.max(left_value));
            points.push((left, current));
            best = Some(current);
            let Some((right, right_value)) = next else {
                break;
            };
            if right_value > current {
                if left_value < current
                    && let Some(fraction) = (current - left_value).div(right_value - left_value)
                {
                    points.push((left + (right - left) * fraction, current));
                }
                best = Some(right_value);
            }
        }
        let last_point = self.last_point();
        let last_value = self.at(last_point);
        let current = best.unwrap_or(last_value);
        let tail = if self.tail > Ratio::zero() {
            if last_value < current
                && let Some(step) = (current - last_value).div(self.tail)
            {
                points.push((last_point + step, current));
            }
            self.tail
        } else {
            Ratio::zero()
        };
        let mut sorted = points;
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.dedup_by(|a, b| a.0 == b.0);
        Piecewise {
            points: sorted,
            tail,
        }
        .simplified()
    }

    pub fn compose(outer: &Piecewise, inner: &Piecewise) -> Piecewise {
        let mut points: Vec<Ratio> = inner.breakpoints().collect();
        let levels: Vec<Ratio> = outer.breakpoints().collect();
        let mut segments: Vec<(Ratio, Ratio, Option<Ratio>, Ratio)> = Vec::new();
        for (index, &(point, value)) in inner.points.iter().enumerate() {
            match inner.points.get(index + 1) {
                Some(&(next, next_value)) => segments.push((point, value, Some(next), next_value)),
                None => segments.push((point, value, None, inner.tail)),
            }
        }
        for (left, left_value, right, right_info) in segments {
            for level in &levels {
                match right {
                    Some(right) => {
                        let right_value = right_info;
                        let between = (left_value < *level && *level < right_value)
                            || (right_value < *level && *level < left_value);
                        if between
                            && let Some(fraction) =
                                (*level - left_value).div(right_value - left_value)
                        {
                            points.push(left + (right - left) * fraction);
                        }
                    }
                    None => {
                        let slope = right_info;
                        let ahead = (slope > Ratio::zero() && *level > left_value)
                            || (slope.is_negative() && *level < left_value);
                        if ahead && let Some(step) = (*level - left_value).div(slope) {
                            points.push(left + step);
                        }
                    }
                }
            }
        }
        points.sort();
        points.dedup();
        let last = points.last().copied().unwrap_or(Ratio::zero());
        let argument = inner.at(last);
        let probe = argument + Ratio::int(1);
        let outer_slope = outer.at(probe) - outer.at(argument);
        let tail = if inner.tail.is_zero() {
            Ratio::zero()
        } else {
            outer_slope * inner.tail
        };
        Piecewise::from_points(points, |t| outer.at(inner.at(t)), tail)
    }

    pub fn zero_run(&self) -> Option<Ratio> {
        let mut last_zero = None;
        for &(point, value) in &self.points {
            if !value.is_zero() {
                return Some(last_zero.unwrap_or(Ratio::zero()));
            }
            last_zero = Some(point);
        }
        if self.tail.is_zero() {
            None
        } else {
            Some(last_zero.unwrap_or(Ratio::zero()))
        }
    }
}

fn interpolate(left: (Ratio, Ratio), right: (Ratio, Ratio), t: Ratio) -> Ratio {
    match (t - left.0).div(right.0 - left.0) {
        Some(fraction) => left.1 + (right.1 - left.1) * fraction,
        None => left.1,
    }
}
