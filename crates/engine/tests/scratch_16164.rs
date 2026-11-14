use serde_json::json;

fn run(s: &mut solidcraft_engine::Session, id: &str, p: serde_json::Value) -> serde_json::Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

fn dump(s: &solidcraft_engine::Session, what: &str) {
    let id = s.active_sketch.unwrap();
    let sk = s.model.state().sketch(id).unwrap().sketch.clone();
    eprintln!("--- {what}");
    for (i, p) in sk.points.iter().enumerate() {
        eprintln!("  point {i} id={} pos={:?} fixed={} link={:?} exprs={:?}", p.id, p.pos, p.fixed, p.link, p.exprs.is_some());
    }
    for (i, c) in sk.curves.iter().enumerate() {
        eprintln!("  curve {i} id={} kind={:?} link={:?} fixed={}", c.id, c.kind, c.link, c.fixed);
    }
    for c in sk.constraints.iter() {
        eprintln!("  constraint {:?} param={:?}", c.kind, c.param);
    }
}

#[test]
fn scratch_fixed_line() {
    let mut s = solidcraft_engine::Session::default();
    run(&mut s, "sketch.create", json!({"plane": "XY"}));
    run(&mut s, "sketch.line", json!({"points": [[5, 0], [15, 0]], "ids": ["l1"]}));
    run(&mut s, "sketch.constraint.fix", json!({"entity": "l1.start"}));
    dump(&s, "line with a fixed start");
    let r = s.execute("sketch.move", &json!({"entities": ["l1"], "translate": [6.0, 0.0]}));
    eprintln!("move the line: {:?}", r.err().map(|e| e.to_string()));
    dump(&s, "after moving the line");
}

#[test]
fn scratch_fixed_circle() {
    let mut s = solidcraft_engine::Session::default();
    run(&mut s, "sketch.create", json!({"plane": "XY"}));
    run(&mut s, "sketch.circle.center", json!({"center": [10, 0], "radius": 5, "id": "c"}));
    run(&mut s, "sketch.constraint.fix", json!({"entity": "c.center"}));
    dump(&s, "circle with a fixed centre");
    let r = s.execute("sketch.move", &json!({"entities": ["c"], "translate": [6.0, 0.0]}));
    eprintln!("move the circle: {:?}", r.err().map(|e| e.to_string()));
    dump(&s, "after moving the circle");
}

#[test]
fn scratch_projected() {
    let mut s = solidcraft_engine::Session::default();
    run(&mut s, "sketch.create", json!({"plane": "XY"}));
    run(&mut s, "sketch.line", json!({"points": [[0, 0], [10, 0]], "ids": ["l1"]}));
    run(&mut s, "sketch.finish", json!({}));
    let src = s.doc.features.last().unwrap().id;
    run(&mut s, "sketch.create", json!({"plane": "XY"}));
    let projected = run(&mut s, "sketch.project_to_surface", json!({"curves":[{"sketch":src,"curve":"l1"}],"face":[0,0,20]}));
    eprintln!("projected: {projected}");
    dump(&s, "projected");
}
