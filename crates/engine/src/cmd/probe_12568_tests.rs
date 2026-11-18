//! Diagnostics for #12568, second symptom: deleting a sketch also deleted a body.
//!
//! TEMPORARY probe file: it prints numbers, it asserts nothing about the fix yet.
use crate::Session;
use serde_json::{Value, json};
use solidcore_record::Vec3;
use solidcraft_geom::prelude::*;

fn run(s: &mut Session, command: &str, params: Value) -> Value {
    s.execute(command, &params).unwrap_or_else(|error| panic!("{command} {params}: {error}"))
}
fn top_pick(s: &Session, body: &str) -> Value {
    let world = s.world_state().unwrap();
    let body = world.body(body).unwrap();
    let face = body
        .body
        .faces(0.01)
        .unwrap()
        .into_iter()
        .filter(|face| face.plane_normal.is_some_and(|normal| normal.dot(Vec3::Z) > 0.999))
        .max_by(|a, b| a.centroid.z.total_cmp(&b.centroid.z))
        .unwrap();
    json!({"body":body.name,"index":face.index,"face":[face.centroid.x,face.centroid.y,face.centroid.z]})
}
fn draw(s: &mut Session) {
    run(s, "sketch.rectangle.two_point", json!({"p0":[1,1],"p1":[3,3]}));
    run(s, "sketch.dimension", json!({"entities":["l1"],"value":2}));
    run(s, "sketch.finish", json!({}));
}

/// Print the document timeline with each feature's kind summary and result error.
fn features_state(s: &Session, label: &str) {
    println!("--- {label}: {} features", s.doc.features.len());
    for feature in &s.doc.features {
        let result = s.model.result(feature.id);
        println!(
            "   feature {} {:?} kind={} -> {:?}",
            feature.id,
            feature.name,
            kind_name(&feature.kind),
            result.map(|r| (r.error.clone(), r.warning.clone())),
        );
    }
}

fn kind_name(kind: &solidcraft_doc::FeatureKind) -> &'static str {
    use solidcraft_doc::FeatureKind::*;
    match kind {
        Sketch { .. } => "Sketch",
        Extrude { .. } => "Extrude",
        Box { .. } => "Box",
        Combine { .. } => "Combine",
        Remove { .. } => "Remove",
        _ => "other",
    }
}

/// Print the bodies in the model with the feature that made each.
fn bodies_state(s: &Session, label: &str) {
    let state = s.model.state();
    let names: Vec<String> = state.bodies.iter().map(|b| b.name.clone()).collect();
    println!("--- {label}: {} bodies {names:?}", state.bodies.len());
    for body in &state.bodies {
        println!("   body {:?} made by feature {} faces {}", body.name, body.feature, body.body.face_count());
    }
}

/// The issue's shape: ONE body, many sketches on its faces, and a sketch deleted while
/// another one is anchored on the body it deletes.
///
/// The design in #12568 has 12 features and 1 body. Deleting the invalidated sketches
/// took the body with it. Here: a box, two sketches on its top face, each extruded into
/// the box. Deleting the FIRST sketch must leave the box and the second sketch alone.
#[test]
fn probe_12568_delete_one_sketch_takes_the_body_the_other_is_anchored_to() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"body_name":"Base","name":"BoxFeature"}));
    let pick = top_pick(&s, "Base");
    let s1 = run(&mut s, "sketch.create", json!({"name":"S1","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s1,"distance":5,"operation":"join","targets":["Base"],"name":"Boss1"}));
    let pick = top_pick(&s, "Base");
    let s2 = run(&mut s, "sketch.create", json!({"name":"S2","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s2,"distance":5,"operation":"join","targets":["Base"],"name":"Boss2"}));
    bodies_state(&s, "before the delete");
    features_state(&s, "before the delete");

    let deleted = run(&mut s, "timeline.delete", json!({"features":[s1]}));
    println!("   deleted reply: {deleted:?}");
    bodies_state(&s, "after deleting the first sketch");
    features_state(&s, "after deleting the first sketch");
    let _ = s2;
}

/// The issue's shape, with a body that is its OWN (not joined): the sketch is on the top face
/// of "Base", and a SECOND sketch on the top face of "Boss" (a separate body). Deleting the
/// first sketch removes the extrude that made "Boss" -- so "Boss" goes, and with it the
/// second sketch, which was anchored to a face of "Boss".
///
/// Is that right? The extrude that made "Boss" is a dependent of the deleted sketch, so it
/// goes with it; "Boss" is its body, so it goes too; and the sketch anchored to a face of
/// "Boss" can no longer be anchored, so it fails. That is a CHAIN of consequences, not a
/// bug in itself.
///
/// The bug the issue reports is different: a body that NOTHING in the deleted chain made.
#[test]
fn probe_12568_delete_sketch_leaves_a_body_it_did_not_make() {
    let mut s = Session::default();
    // "Base" is made by a box. "Other" is made by another box, far away.
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"body_name":"Base","name":"BoxFeature"}));
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"corner":[100,0,0],"body_name":"Other","name":"OtherFeature"}));
    // A sketch on Base's top face, extruded into a NEW body "Boss".
    let pick = top_pick(&s, "Base");
    let s1 = run(&mut s, "sketch.create", json!({"name":"S1","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s1,"distance":5,"operation":"new","body_name":"Boss","name":"BossFeature"}));
    bodies_state(&s, "before the delete");
    features_state(&s, "before the delete");

    let deleted = run(&mut s, "timeline.delete", json!({"features":[s1]}));
    println!("   deleted reply: {deleted:?}");
    bodies_state(&s, "after deleting the sketch");
    features_state(&s, "after deleting the sketch");
}

/// The issue's shape with an undo and a redo before the delete (the user "redid a sketch").
#[test]
fn probe_12568_delete_after_undo_redo() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"body_name":"Base","name":"BoxFeature"}));
    let pick = top_pick(&s, "Base");
    let s1 = run(&mut s, "sketch.create", json!({"name":"S1","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s1,"distance":5,"operation":"new","body_name":"Boss","name":"BossFeature"}));
    run(&mut s, "edit.undo", json!({}));
    run(&mut s, "edit.redo", json!({}));
    bodies_state(&s, "after undo+redo");
    features_state(&s, "after undo+redo");

    let deleted = run(&mut s, "timeline.delete", json!({"features":[s1]}));
    println!("   deleted reply: {deleted:?}");
    bodies_state(&s, "after deleting the sketch");
    features_state(&s, "after deleting the sketch");
}

/// Deleting a sketch whose body was JOINED into another: the join (a Combine) names the
/// body by NAME, so the Combine is not a dependent of the sketch and stays in the timeline.
/// It then fails with "unknown body" -- the case #267 closed.
#[test]
fn probe_12568_delete_sketch_leaves_the_combine_that_names_the_body() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"body_name":"Base","name":"BoxFeature"}));
    let pick = top_pick(&s, "Base");
    let s1 = run(&mut s, "sketch.create", json!({"name":"S1","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s1,"distance":5,"operation":"new","body_name":"Boss","name":"BossFeature"}));
    run(&mut s, "solid.combine", json!({"target":"Base","tools":["Boss"],"operation":"join","name":"Join"}));
    bodies_state(&s, "after the join");
    features_state(&s, "after the join");

    let deleted = run(&mut s, "timeline.delete", json!({"features":[s1]}));
    println!("   deleted reply: {deleted:?}");
    bodies_state(&s, "after deleting the sketch");
    features_state(&s, "after deleting the sketch");
}

/// What the UI shows before the delete: `timeline.dependents` lists what goes with it.
#[test]
fn probe_12568_dependents_report() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"body_name":"Base","name":"BoxFeature"}));
    let pick = top_pick(&s, "Base");
    let s1 = run(&mut s, "sketch.create", json!({"name":"S1","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s1,"distance":5,"operation":"new","body_name":"Boss","name":"BossFeature"}));
    let pick = top_pick(&s, "Boss");
    let s2 = run(&mut s, "sketch.create", json!({"name":"S2","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s2,"distance":5,"operation":"new","body_name":"Boss2","name":"Boss2Feature"}));
    let deps = run(&mut s, "timeline.dependents", json!({"feature": s1}));
    println!("   dependents of S1: {deps:?}");
    let deps = run(&mut s, "timeline.dependents", json!({"feature": s2}));
    println!("   dependents of S2: {deps:?}");
    let _ = s2;
}

// ---------------------------------------------------------------------------
// The mechanisms that could take a body the user did not name away.
// ---------------------------------------------------------------------------

/// `solid.remove` adds a `Remove { bodies }` feature: a body is gone from the model from
/// that point on, but the feature that MADE it stays. Deleting the sketch that made it
/// then leaves the `Remove` naming a body that no longer exists -- or, worse, removes a
/// body of the same name that came later.
#[test]
fn probe_12568_remove_feature_and_a_deleted_sketch() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"body_name":"Base","name":"BoxFeature"}));
    let pick = top_pick(&s, "Base");
    let s1 = run(&mut s, "sketch.create", json!({"name":"S1","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s1,"distance":5,"operation":"new","body_name":"Boss","name":"BossFeature"}));
    // The user removes the body "Boss" (a Remove feature in the timeline).
    run(&mut s, "solid.remove", json!({"bodies":["Boss"],"name":"RemoveBoss"}));
    bodies_state(&s, "after the remove");
    features_state(&s, "after the remove");

    let deleted = run(&mut s, "timeline.delete", json!({"features":[s1]}));
    println!("   deleted reply: {deleted:?}");
    bodies_state(&s, "after deleting the sketch");
    features_state(&s, "after deleting the sketch");
}

/// The `Remove` feature names a body by NAME. After the sketch that made it is deleted,
/// the name is free again -- and a body made later may take it, so the stale `Remove`
/// takes away a body the user still has.
#[test]
fn probe_12568_remove_feature_binds_a_later_body_of_the_same_name() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"body_name":"Base","name":"BoxFeature"}));
    let pick = top_pick(&s, "Base");
    let s1 = run(&mut s, "sketch.create", json!({"name":"S1","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s1,"distance":5,"operation":"new","body_name":"Boss","name":"BossFeature"}));
    run(&mut s, "solid.remove", json!({"bodies":["Boss"],"name":"RemoveBoss"}));
    // Delete the sketch: the extrude that made "Boss" goes with it.
    let deleted = run(&mut s, "timeline.delete", json!({"features":[s1]}));
    println!("   deleted reply: {deleted:?}");
    bodies_state(&s, "after deleting the sketch");
    features_state(&s, "after deleting the sketch");
    // A new body takes the name "Boss".
    let pick = top_pick(&s, "Base");
    let s2 = run(&mut s, "sketch.create", json!({"name":"S2","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s2,"distance":5,"operation":"new","body_name":"Boss","name":"BossAgain"}));
    bodies_state(&s, "after a new body takes the name");
    features_state(&s, "after a new body takes the name");
}

/// A `Pattern` copies a feature's bodies. Deleting the sketch the source was built from
/// takes the source away; the Pattern then makes nothing -- do its bodies go too, and does
/// the reply say so?
#[test]
fn probe_12568_pattern_of_a_deleted_sketch() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":20,"width":20,"height":10,"body_name":"Base","name":"BoxFeature"}));
    let pick = top_pick(&s, "Base");
    let s1 = run(&mut s, "sketch.create", json!({"name":"S1","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":s1,"distance":5,"operation":"new","body_name":"Boss","name":"BossFeature"}));
    let out = run(&mut s, "solid.pattern.rectangular", json!({"features":["BossFeature"],"distance":[30,0,0],"count":2,"name":"Pattern"}));
    println!("   pattern reply: {out:?}");
    bodies_state(&s, "after the pattern");
    features_state(&s, "after the pattern");

    let deleted = run(&mut s, "timeline.delete", json!({"features":[s1]}));
    println!("   deleted reply: {deleted:?}");
    bodies_state(&s, "after deleting the sketch");
    features_state(&s, "after deleting the sketch");
}

/// The issue's design has 12 features and ONE body. So the body the user lost was the
/// design's only body. Reproduce that shape: a box, a shell, a fillet, a boss and a join
/// (the #4325 recipe), then a sketch on the rim, then delete the sketch.
#[test]
fn probe_12568_the_4325_recipe_with_a_sketch_deleted() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":40,"width":20,"height":20,"body_name":"Body1"}));
    run(&mut s, "solid.shell", json!({"faces":[[20,10,20]],"thickness":1}));
    run(&mut s, "solid.fillet", json!({"edges":[[20,1,1]],"radius":2}));
    run(&mut s, "solid.cylinder", json!({"radius":3,"height":19,"base":[10,10,1],"operation":"new","body_name":"Boss"}));
    run(&mut s, "solid.combine", json!({"target":"Body1","tools":["Boss"],"operation":"join"}));
    bodies_state(&s, "after the join");
    features_state(&s, "after the join");
    let deps = run(&mut s, "timeline.dependents", json!({"feature": 2}));
    println!("   dependents of feature 2: {deps:?}");
}
