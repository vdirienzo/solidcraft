use crate::scenario::Harness;
use serde_json::json;

#[test]
fn repro_linked_first_point() {
    let mut h = Harness::new();
    h.app.set_origin_shown(false);
    h.app.run("solid.box", json!({"length":40,"width":10,"height":10,"corner":[0,0,-20]})).unwrap();
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.app.run("sketch.project", json!({"refs":[{"edge":[20,0,-10]}]})).unwrap();
    h.app.cam.distance = 100.0;
    h.settle();
    let sk = h.app.session.doc.sketch(h.app.session.active_sketch.unwrap()).unwrap();
    println!("curves: {:?}", sk.curves);
    h.step(&json!({"start":"sketch.line"})).unwrap();
    println!("tool after start: {:?}", h.app.tool);
    let p = h.call("ui.at", json!({"sketch":[2,0]}))["result"].clone();
    println!("screen point: {p}");
    h.call("ui.move", json!({"x":p[0],"y":p[1]}));
    h.settle();
    println!("tool after move: {:?}", h.app.tool);
    h.call("ui.click", json!({"x":p[0],"y":p[1]}));
    h.settle();
    println!("tool after click: {:?}", h.app.tool);
    println!("status: {:?}", h.app.status);
    let state = h.app.session.world_state().unwrap();
    println!("sketch curves: {:?}", state.sketches[0].sketch.curves);
    println!("sketch points: {:?}", state.sketches[0].sketch.points);
    println!("sketch constraints: {:?}", state.sketches[0].sketch.constraints);
}
