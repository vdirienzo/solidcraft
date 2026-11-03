//! The Line tool starts a line from a snap point of the geometry already in the sketch
//! (#11171, #11186).
#![allow(clippy::unwrap_used)]
use crate::scenario::Harness;
use serde_json::{Value, json};

fn step(h: &mut Harness, value: Value) {
    h.step(&value).unwrap();
}

/// A sketch with one line, so its midpoint and end points are snap points.
fn fixture() -> Harness {
    let mut h = Harness::new();
    h.app.preferences.look_at_anim = false;
    for (command, params) in [("sketch.create", json!({"plane":"XY"})), ("sketch.line", json!({"points":[[-30,10],[-15,10]]}))] {
        step(&mut h, json!({"call":"engine.execute","params":{"command":command,"params":params}}));
    }
    step(&mut h, json!({"call":"ui.view","params":{"view":"top"}}));
    h
}

/// The Line tool starts a line when the first click lands on a snap point of the geometry
/// already in the sketch: the midpoint of a line and the end point of a line.
#[test]
fn the_line_tool_starts_from_a_snap_point() {
    for (name, at) in [("the midpoint of the line", [-22.5, 10.0]), ("the end of the line", [-15.0, 10.0])] {
        let mut h = fixture();
        step(&mut h, json!({"start":"sketch.line"}));
        step(&mut h, json!({"click":{"sketch":at}}));
        let tool = h.app.tool.as_ref().expect("the Line tool is still up");
        assert_eq!(tool.pts.len(), 1, "{name}: the click did not start the line");
        // The second click finishes the line.
        step(&mut h, json!({"click":{"sketch":[10,30]}}));
        let sk = h.app.session.doc.sketch(h.app.session.active_sketch.unwrap()).unwrap();
        assert_eq!(sk.curves.len(), 2, "{name}: the line was not drawn");
    }
}
