//! Distributing objects along a span (#9866): the maths of fitting N objects between two ends.
//!
//! A span is the stretch between two points that differ along one axis. The objects are laid
//! along it, in the order they are given, each one taking its own size along that axis. There
//! are two ways of spreading them: the gaps between the objects are all equal
//! ([`Mode::Gaps`]), or their centres are equally spaced ([`Mode::Centers`]).
//!
//! The two ways agree only when the objects are all the same size: equal gaps and equal centre
//! spacing are one and the same layout for equal objects, and two different layouts for
//! unequal ones. With equal objects both ways step by `(span - object) / (count - 1)`; with
//! unequal ones the gaps share out the room the span has left over, while the centres step
//! evenly from the first object to the last.
//!
//! The objects have to fit: their sizes added up may not be longer than the span, or there is
//! no room for them. A span too short is refused, with the numbers that say so; nothing is
//! squeezed or clamped to make it fit.

use crate::{DocError, Result};

/// How close two lengths have to be to count as the same length (mm).
const LENGTH_EPS: f64 = 1e-9;

/// How the objects are spread along a span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The gaps between the objects are all equal: the room the span has left over is shared
    /// out evenly between them, and the first object sits at the start of the span.
    Gaps,
    /// The centres of the objects are equally spaced: the first centre sits half the first
    /// object's size from the start of the span, the last half the last object's size from its
    /// end, and the centres in between step evenly from one to the other.
    Centers,
}

/// The objects laid along the span between `from` and `to`, in the order of `sizes`.
///
/// `from` and `to` are the ends of the span: they must differ along exactly one axis, and that
/// axis is the one the objects are laid along. Each size is how long its object is along that
/// axis. The answer is where each object begins, on the side of the span it starts from: the
/// first object begins at `from`, and, when there is more than one, the last one ends at `to`.
/// The other two coordinates of every answer are those of `from`, the span running along one
/// axis only.
///
/// The objects have to fit: their sizes added up may not be longer than the span. A span too
/// short is refused, and so is a span whose ends do not run along one axis.
pub fn distribute(from: [f64; 3], to: [f64; 3], sizes: &[f64], mode: Mode) -> Result<Vec<[f64; 3]>> {
    let axis = span_axis(from, to)?;
    let along = distribute_along(from[axis], to[axis], sizes, mode)?;
    Ok(along
        .into_iter()
        .map(|value| {
            let mut point = from;
            point[axis] = value;
            point
        })
        .collect())
}

/// The objects laid along one axis: where each object begins, measured on that axis.
///
/// The span runs from `from` to `to`, and the objects are laid in the order of `sizes`, each
/// one taking its own size along the axis. The answer is where each object begins, on the side
/// of the span it starts from: the first object begins at `from`, and, when there is more than
/// one, the last one ends at `to`. A span that runs backwards (its end below its start) is
/// laid out backwards too, so that the first object is still the one at `from`.
///
/// The objects have to fit: their sizes added up may not be longer than the span.
pub fn distribute_along(from: f64, to: f64, sizes: &[f64], mode: Mode) -> Result<Vec<f64>> {
    if !from.is_finite() || !to.is_finite() {
        return Err(DocError::Invalid("a span's ends must be finite numbers".into()));
    }
    if sizes.is_empty() {
        return Err(DocError::Invalid("there are no objects to distribute".into()));
    }
    if sizes.iter().any(|size| !size.is_finite() || *size < 0.0) {
        return Err(DocError::Invalid("an object's size along the span must be a finite length".into()));
    }
    let count = sizes.len();
    let length = (to - from).abs();
    let total: f64 = sizes.iter().sum();
    if total > length + LENGTH_EPS {
        return Err(DocError::Invalid(format!("the objects do not fit in the span: they need {total:.3} mm but the span is {length:.3} mm")));
    }
    // The way the span runs: the objects follow it, from `from` towards `to`.
    let along = if to >= from { 1.0 } else { -1.0 };
    // Where each object begins, measured from the start of the span.
    let begins = match mode {
        // The room left over, shared out evenly between the gaps. With one object there are
        // no gaps at all, so it simply sits at the start of the span.
        Mode::Gaps => {
            // The room left over is shared out evenly. The clamp only takes the dust off a gap
            // that is zero to within a hair: a gap that is really negative is refused above.
            let gap = if count > 1 { ((length - total) / (count as f64 - 1.0)).max(0.0) } else { 0.0 };
            let mut begins = Vec::with_capacity(count);
            let mut at = 0.0;
            for size in sizes {
                begins.push(at);
                at += size + gap;
            }
            begins
        }
        // The centres step evenly from the first one, half the first object's size in, to the
        // last one, half the last object's size short of the end. With one object there is
        // only the first centre, and the object begins at the start of the span.
        Mode::Centers => {
            let first = sizes[0] / 2.0;
            let step = if count > 1 { (length - (sizes[0] + sizes[count - 1]) / 2.0) / (count as f64 - 1.0) } else { 0.0 };
            sizes.iter().enumerate().map(|(index, size)| first + index as f64 * step - size / 2.0).collect()
        }
    };
    Ok(begins.into_iter().map(|at| from + along * at).collect())
}

/// The axis a span runs along: the one its two ends differ on.
fn span_axis(from: [f64; 3], to: [f64; 3]) -> Result<usize> {
    if from.iter().chain(to.iter()).any(|value| !value.is_finite()) {
        return Err(DocError::Invalid("a span's ends must be finite numbers".into()));
    }
    let mut differing = Vec::new();
    for (axis, (start, end)) in from.iter().zip(to.iter()).enumerate() {
        if (end - start).abs() > LENGTH_EPS {
            differing.push(axis);
        }
    }
    match differing.len() {
        1 => Ok(differing[0]),
        0 => Err(DocError::Invalid("a span's two ends are the same: it must run along one axis".into())),
        many => Err(DocError::Invalid(format!("a span must run along one axis, but its ends differ along {many} axes"))),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;

    /// Whether two lengths are the same to within a hair (mm).
    fn close(got: f64, want: f64) -> bool {
        (got - want).abs() < LENGTH_EPS
    }

    /// Whether two runs of lengths are the same to within a hair (mm).
    fn close_all(got: &[f64], want: &[f64]) -> bool {
        got.len() == want.len() && got.iter().zip(want).all(|(a, b)| close(*a, *b))
    }

    #[test]
    fn one_object_begins_where_the_span_begins() {
        let from = [0.0, 0.0, 0.0];
        let to = [10.0, 0.0, 0.0];
        // With one object there are no gaps to leave, so it sits at the start of the span.
        let gaps = distribute(from, to, &[4.0], Mode::Gaps).unwrap();
        assert_eq!(gaps, [[0.0, 0.0, 0.0]]);
        // Its centre is half its own size along the span.
        let centers = distribute(from, to, &[4.0], Mode::Centers).unwrap();
        assert_eq!(centers, [[0.0, 0.0, 0.0]]);
        assert!(close(centers[0][0] + 2.0, 2.0));
    }

    #[test]
    fn equal_objects_land_where_both_ways_agree() {
        let from = [0.0, 0.0, 0.0];
        let to = [100.0, 0.0, 0.0];
        let gaps = distribute(from, to, &[10.0; 3], Mode::Gaps).unwrap();
        let centers = distribute(from, to, &[10.0; 3], Mode::Centers).unwrap();
        // Equal gaps and equal centre spacing are the same layout for equal objects.
        assert!(close_all(&gaps.iter().map(|point| point[0]).collect::<Vec<_>>(), &[0.0, 45.0, 90.0]));
        assert!(close_all(&centers.iter().map(|point| point[0]).collect::<Vec<_>>(), &[0.0, 45.0, 90.0]));
        // The step is (span - object) / (count - 1).
        assert!(close(45.0, (100.0 - 10.0) / 2.0));
        // The last object ends at the end of the span.
        assert!(close(gaps[2][0] + 10.0, 100.0));
    }

    #[test]
    fn unequal_objects_split_the_two_ways() {
        let from = [0.0, 0.0, 0.0];
        let to = [100.0, 0.0, 0.0];
        let sizes = [10.0, 20.0, 30.0];
        let gaps = distribute(from, to, &sizes, Mode::Gaps).unwrap();
        let centers = distribute(from, to, &sizes, Mode::Centers).unwrap();
        let gap_starts = gaps.iter().map(|point| point[0]).collect::<Vec<_>>();
        let center_starts = centers.iter().map(|point| point[0]).collect::<Vec<_>>();
        // The two ways lay the same objects out differently.
        assert!(close_all(&gap_starts, &[0.0, 30.0, 70.0]));
        assert!(close_all(&center_starts, &[0.0, 35.0, 70.0]));
        assert!(!close_all(&gap_starts, &center_starts));
        // The gaps are equal: the room left over (40) shared between two gaps.
        let gap = (100.0 - 60.0) / 2.0;
        assert!(close(gap, 20.0));
        assert!(close(gap_starts[1] - (gap_starts[0] + sizes[0]), gap));
        assert!(close(gap_starts[2] - (gap_starts[1] + sizes[1]), gap));
        // The centres are equally spaced: 5, 45 and 85.
        let gap_centers = sizes.iter().zip(&gap_starts).map(|(size, start)| start + size / 2.0).collect::<Vec<_>>();
        let center_centers = sizes.iter().zip(&center_starts).map(|(size, start)| start + size / 2.0).collect::<Vec<_>>();
        assert!(close_all(&gap_centers, &[5.0, 40.0, 85.0]));
        assert!(close_all(&center_centers, &[5.0, 45.0, 85.0]));
        assert!(close(center_centers[1] - center_centers[0], center_centers[2] - center_centers[1]));
    }

    #[test]
    fn objects_that_do_not_fit_are_refused() {
        let from = [0.0, 0.0, 0.0];
        let to = [10.0, 0.0, 0.0];
        let error = distribute(from, to, &[6.0, 6.0], Mode::Gaps).unwrap_err().to_string();
        assert!(error.contains("do not fit"), "{error}");
        // The same objects are refused the other way round too.
        assert!(distribute(from, to, &[6.0, 6.0], Mode::Centers).is_err());
        // One object too long for the span is refused as well.
        assert!(distribute(from, to, &[12.0], Mode::Gaps).is_err());
    }

    #[test]
    fn a_span_must_run_along_one_axis() {
        let from = [0.0, 0.0, 0.0];
        // Two axes differ: there is no one axis to lay the objects along.
        let error = distribute(from, [10.0, 10.0, 0.0], &[4.0], Mode::Gaps).unwrap_err().to_string();
        assert!(error.contains("one axis"), "{error}");
        // No axis differs: the span has no length at all.
        let error = distribute(from, from, &[4.0], Mode::Gaps).unwrap_err().to_string();
        assert!(error.contains("one axis"), "{error}");
        // Each axis on its own is a span.
        assert!(distribute(from, [0.0, 10.0, 0.0], &[4.0], Mode::Gaps).is_ok());
        assert!(distribute(from, [0.0, 0.0, 10.0], &[4.0], Mode::Gaps).is_ok());
    }

    #[test]
    fn a_span_of_no_length_does_not_divide_by_zero() {
        // One object in a span of no length: there is no gap to leave and nothing to divide.
        // The object does not fit in a span of no length, and is told so rather than divided by.
        assert!(distribute_along(0.0, 0.0, &[1.0], Mode::Gaps).is_err());
        assert!(distribute_along(0.0, 0.0, &[1.0], Mode::Centers).is_err());
        // Objects of no size in a span of no length: they fit, and there is still no division.
        assert_eq!(distribute_along(0.0, 0.0, &[0.0, 0.0], Mode::Gaps).unwrap(), [0.0, 0.0]);
        assert_eq!(distribute_along(0.0, 0.0, &[0.0, 0.0], Mode::Centers).unwrap(), [0.0, 0.0]);
        // Several objects in a span of no length: they cannot fit, and are told so.
        let error = distribute_along(0.0, 0.0, &[1.0, 2.0], Mode::Gaps).unwrap_err().to_string();
        assert!(error.contains("do not fit"), "{error}");
        assert!(distribute_along(0.0, 0.0, &[1.0, 2.0], Mode::Centers).is_err());
    }

    #[test]
    fn a_span_running_backwards_is_laid_out_backwards() {
        let from = [100.0, 0.0, 0.0];
        let to = [0.0, 0.0, 0.0];
        let gaps = distribute(from, to, &[10.0; 3], Mode::Gaps).unwrap();
        assert!(close_all(&gaps.iter().map(|point| point[0]).collect::<Vec<_>>(), &[100.0, 55.0, 10.0]));
        let centers = distribute(from, to, &[10.0; 3], Mode::Centers).unwrap();
        assert!(close_all(&centers.iter().map(|point| point[0]).collect::<Vec<_>>(), &[100.0, 55.0, 10.0]));
    }

    #[test]
    fn objects_are_refused_when_they_cannot_be_told_apart() {
        let from = [0.0, 0.0, 0.0];
        let to = [10.0, 0.0, 0.0];
        // There are no objects to lay out.
        assert!(distribute(from, to, &[], Mode::Gaps).is_err());
        // A size is not a length.
        assert!(distribute(from, to, &[-1.0], Mode::Gaps).is_err());
        assert!(distribute(from, to, &[f64::NAN], Mode::Gaps).is_err());
        // The ends of the span are not numbers.
        assert!(distribute(from, [f64::NAN, 0.0, 0.0], &[1.0], Mode::Gaps).is_err());
    }
}
