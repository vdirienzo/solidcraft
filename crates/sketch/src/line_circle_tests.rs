//! The distance from a line to the edge of a circle or arc (#1803): solved numerically and by
//! construction, measured, kept on its side of the line through edits, and refused on other
//! kinds of curve.

use solidcore_record::Vec2;
use solidcraft_geom::prelude::*;

use crate::tests::v;
use crate::*;

/// A fixed line along x from (0, 0) to (40, 0) and a circle of radius `r` at `centre` (its
/// centre free, its radius set by a dimension). The line, the circle and the centre point.
fn line_and_circle(centre: Vec2, r: f64) -> (Sketch, usize, usize, usize) {
    let mut sk = Sketch::new();
    let l = sk.add_line(v(0.0, 0.0), v(40.0, 0.0), None, None, None).unwrap();
    let (a, b) = sk.curves[l].kind.ends().unwrap();
    for p in [a, b] {
        sk.add_constraint(ConstraintKind::Fix { p }, None).unwrap();
    }
    let c = sk.add_circle(centre, r, None, None).unwrap();
    let CurveKind::Circle { c: ci, .. } = sk.curves[c].kind else { panic!("not a circle") };
    sk.add_constraint(ConstraintKind::Radius { c, value: r }, None).unwrap();
    (sk, l, c, ci)
}

fn kind_at(sk: &Sketch, k: usize) -> ConstraintKind {
    sk.constraints[k].kind.clone()
}

#[test]
fn a_line_to_circle_distance_places_the_circle_its_gap_from_the_line() {
    let (mut sk, l, c, ci) = line_and_circle(v(20.0, 15.0), 5.0);
    let k = sk.constraints.len();
    sk.add_constraint(ConstraintKind::LineCircleDistance { l, c, value: 10.0, inside: false }, None).unwrap();
    // As drawn the gap is 15 − 5 = 10: the sketch already holds it.
    assert!((solver::measure_dimension(&sk, &kind_at(&sk, k)).unwrap() - 10.0).abs() < 1e-9);
    assert!(solve(&mut sk).ok());
    // A gap of 7 puts the centre 7 + 5 = 12 from the line, on the side it was drawn on.
    sk.constraints[k].kind.set_value(7.0);
    let rep = solve(&mut sk);
    assert!(rep.ok(), "{rep:?}");
    let centre = sk.points[ci].pos;
    assert!((centre.y - 12.0).abs() < 1e-7, "{centre:?}");
    assert!((solver::measure_dimension(&sk, &kind_at(&sk, k)).unwrap() - 7.0).abs() < 1e-7);
    // Tangent: a gap of zero.
    sk.constraints[k].kind.set_value(0.0);
    assert!(solve(&mut sk).ok());
    assert!((sk.points[ci].pos.y - 5.0).abs() < 1e-7, "{:?}", sk.points[ci].pos);
}

#[test]
fn a_circle_below_the_line_stays_below_it() {
    let (mut sk, l, c, ci) = line_and_circle(v(20.0, -15.0), 5.0);
    let k = sk.constraints.len();
    sk.add_constraint(ConstraintKind::LineCircleDistance { l, c, value: 10.0, inside: false }, None).unwrap();
    assert!(solve(&mut sk).ok());
    sk.constraints[k].kind.set_value(3.0);
    assert!(solve(&mut sk).ok());
    assert!((sk.points[ci].pos.y + 8.0).abs() < 1e-7, "{:?}", sk.points[ci].pos);
}

#[test]
fn a_line_that_cuts_the_circle_measures_how_far_its_edge_reaches_past_it() {
    // Centre 3 above the line, radius 5: the edge reaches 5 − 3 = 2 below the line.
    let (mut sk, l, c, ci) = line_and_circle(v(20.0, 3.0), 5.0);
    let k = sk.constraints.len();
    sk.add_constraint(ConstraintKind::LineCircleDistance { l, c, value: 2.0, inside: true }, None).unwrap();
    assert!((solver::measure_dimension(&sk, &kind_at(&sk, k)).unwrap() - 2.0).abs() < 1e-9);
    assert!(solve(&mut sk).ok());
    // Reaching 4 past the line leaves the centre 5 − 4 = 1 above it.
    sk.constraints[k].kind.set_value(4.0);
    let rep = solve(&mut sk);
    assert!(rep.ok(), "{rep:?}");
    assert!((sk.points[ci].pos.y - 1.0).abs() < 1e-7, "{:?}", sk.points[ci].pos);
}

#[test]
fn an_arc_is_measured_to_its_circle() {
    let mut sk = Sketch::new();
    let l = sk.add_line(v(0.0, 0.0), v(40.0, 0.0), None, None, None).unwrap();
    let a = sk.add_arc(v(20.0, 20.0), v(26.0, 20.0), v(14.0, 20.0), [None; 3], None).unwrap();
    let k = sk.constraints.len();
    sk.add_constraint(ConstraintKind::LineCircleDistance { l, c: a, value: 14.0, inside: false }, None).unwrap();
    // Centre 20 above the line, radius 6.
    assert!((solver::measure_dimension(&sk, &kind_at(&sk, k)).unwrap() - 14.0).abs() < 1e-9);
}

#[test]
fn the_dimension_needs_a_line_and_a_circle_or_arc() {
    let mut sk = Sketch::new();
    let l = sk.add_line(v(0.0, 0.0), v(40.0, 0.0), None, None, None).unwrap();
    let m = sk.add_line(v(0.0, 10.0), v(40.0, 10.0), None, None, None).unwrap();
    let c = sk.add_circle(v(20.0, 20.0), 5.0, None, None).unwrap();
    for (l, c) in [(m, l), (c, c), (c, l)] {
        assert!(sk.add_constraint(ConstraintKind::LineCircleDistance { l, c, value: 1.0, inside: false }, None).is_err(), "{l} {c}");
    }
    for value in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(sk.add_constraint(ConstraintKind::LineCircleDistance { l, c, value, inside: false }, None).is_err(), "{value}");
    }
}

#[test]
fn the_dimension_survives_a_round_trip_and_a_renumbering() {
    let (mut sk, l, c, _) = line_and_circle(v(20.0, 3.0), 5.0);
    sk.add_constraint(ConstraintKind::LineCircleDistance { l, c, value: 2.0, inside: true }, None).unwrap();
    let json = serde_json::to_value(&sk).unwrap();
    let saved = json["constraints"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(saved["type"], "line_circle_distance", "{saved}");
    assert_eq!(saved["inside"], true, "{saved}");
    let back: Sketch = serde_json::from_value(json).unwrap();
    assert_eq!(back.constraints, sk.constraints);
    // `inside` is left out when false.
    let outside = ConstraintKind::LineCircleDistance { l, c, value: 2.0, inside: false };
    assert!(serde_json::to_value(&outside).unwrap().get("inside").is_none());
    let mut k = outside;
    assert!(k.remap(&|p| Some(p), &|i| Some(i + 10)));
    assert_eq!(k.indices(), (vec![], vec![l + 10, c + 10]));
    assert!(!k.remap(&|p| Some(p), &|_| None));
}

/// With the centre's x set too, the seed places the centre exactly (its offset line from the
/// dimension crossing the line of its x), before the numerical solver runs.
#[test]
fn the_seed_places_the_centre_by_construction() {
    for (drawn, inside, value, want) in [(v(20.0, 15.0), false, 7.0, v(20.0, 12.0)), (v(20.0, 3.0), true, 4.0, v(20.0, 1.0))] {
        let (mut sk, l, c, ci) = line_and_circle(drawn, 5.0);
        let (a, _) = sk.curves[l].kind.ends().unwrap();
        sk.add_constraint(ConstraintKind::DistanceX { p: a, q: ci, value: 20.0 }, None).unwrap();
        sk.add_constraint(ConstraintKind::LineCircleDistance { l, c, value, inside }, None).unwrap();
        let out = crate::tests::seed_of(&sk);
        assert!(out.placed_by[ci].is_some_and(crate::construct::Placement::anchors), "{out:?}");
        assert!(out.pos[ci].unwrap().dist(want) < 1e-9, "{out:?}");
        let rep = solve(&mut sk);
        assert!(rep.ok() && rep.dof == 0, "{rep:?}");
        assert!(sk.points[ci].pos.dist(want) < 1e-9, "{:?}", sk.points[ci].pos);
    }
}

#[test]
fn tmp_repro_circle_in_angle_stays_inside_when_a_line_is_dimensioned() {
    // The angle: bottom line from a fixed point A to the vertex B, slanted line from B up to a
    // fixed point C. The circle is tangent to both, inside.
    let build = |l0: f64| {
        let mut sk = Sketch::new();
        let a = v(0.0, 0.0);
        let b = v(l0, 0.0);
        let c = v(33.0, 200.0);
        let l1 = sk.add_line(a, b, None, None, None).unwrap();
        let l2 = sk.add_line(b, c, None, None, None).unwrap();
        let (a1, b1) = sk.curves[l1].kind.ends().unwrap();
        let (a2, b2) = sk.curves[l2].kind.ends().unwrap();
        sk.add_constraint(ConstraintKind::Coincident { p: b1, q: a2 }, None).unwrap();
        let cir = sk.add_circle(v(0.0, 0.0), 10.0, None, None).unwrap();
        let CurveKind::Circle { c: ci, .. } = sk.curves[cir].kind else { panic!() };
        sk.points[ci].pos = v(0.0, 0.0);
        sk.add_constraint(ConstraintKind::Radius { c: cir, value: 10.0 }, None).unwrap();
        sk.add_constraint(ConstraintKind::Tangent { a: l1, b: cir }, None).unwrap();
        sk.add_constraint(ConstraintKind::Tangent { a: l2, b: cir }, None).unwrap();
        sk.add_constraint(ConstraintKind::Fix { p: a1 }, None).unwrap();
        sk.add_constraint(ConstraintKind::Fix { p: b2 }, None).unwrap();
        (sk, l1, l2, cir, ci, b1)
    };
    for (l0, ln) in [(110.0_f64, 60.0_f64), (100.0, 60.0), (80.0, 60.0), (60.0, 110.0), (110.0, 100.0)] {
        let (mut sk, l1, _l2, _cir, ci, _b1) = build(l0);
        // Place the centre at radius 10 from both lines, inside: y=10 and the slant's side of A.
        let (p0, p1) = (sk.points[sk.curves[l1].kind.ends().unwrap().0].pos, sk.points[sk.curves[l1].kind.ends().unwrap().1].pos);
        let l2e = sk.curves[1].kind.ends().unwrap();
        let (q0, q1) = (sk.points[l2e.0].pos, sk.points[l2e.1].pos);
        let d = (q1 - q0).normalized().unwrap();
        let n = v(-d.y, d.x);
        // Choose the centre 10 above the bottom line on the side of the slant that holds A.
        let sgn = if n.cross(p0 - q0) < 0.0 { -1.0 } else { 1.0 };
        let line_off = q0 + n * sgn * 10.0;
        // Along the slanted line's normal, at distance 10, pick the point at y=10.
        let m = v(-d.y, d.x) * sgn;
        let _ = (p0, p1, line_off, m);
        // Solve for the centre from the two line distances using a small 2x2 solve.
        let (n1, n2) = (v(0.0, 1.0), { let dir = (q1 - q0).normalized().unwrap(); let nn = v(-dir.y, dir.x); if nn.cross(v(0.0, 0.0) - q0) > 0.0 { nn } else { -nn } });
        // centre = p0 + 10*n1 + t*dir1 ... simpler: centre y=10, slant distance 10.
        let dir = (q1 - q0).normalized().unwrap();
        // (q1-q0) x (centre-q0) = +-10*len
        let want = 10.0 * (q1 - q0).len();
        let s = if n2.cross(p0 - q0) > 0.0 { 1.0 } else { -1.0 };
        // solve x: dir.x*(10) - dir.y*(x-0) = s*want/len ... use cross formula numerically
        // cross = dir.x*(10-q0.y) - dir.y*(x-q0.x)
        let x = (dir.x * (10.0 - q0.y) + dir.y * q0.x - s * 10.0) / dir.y;
        sk.points[ci].pos = v(x, 10.0);
        let mut rep = solve(&mut sk);
        let (s1, s2) = (side_of(&sk, l1, ci), side_of(&sk, 1, ci));
        eprintln!("L0 {l0} -> {ln}: first solve {rep:?} centre {:?} sides {s1} {s2}", sk.points[ci].pos);
        assert!(rep.ok(), "{rep:?}");
        let before = (s1, s2);
        sk.add_constraint(ConstraintKind::Length { l: l1, value: ln }, None).unwrap();
        rep = solve(&mut sk);
        let (t1, t2) = (side_of(&sk, l1, ci), side_of(&sk, 1, ci));
        eprintln!("  after Length {ln}: {rep:?} centre {:?} sides {t1} {t2}", sk.points[ci].pos);
        assert!(rep.ok(), "{rep:?}");
        assert_eq!(before, (t1, t2), "L0 {l0} -> {ln}: circle flipped: {:?}", sk.points[ci].pos);
    }
}

fn side_of(sk: &Sketch, l: usize, ci: usize) -> f64 {
    let (a, b) = sk.curves[l].kind.ends().unwrap();
    (sk.points[b].pos - sk.points[a].pos).cross(sk.points[ci].pos - sk.points[a].pos).signum()
}
