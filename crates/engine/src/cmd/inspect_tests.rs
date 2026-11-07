use serde_json::{Value, json};

use crate::Session;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

fn param<'a>(inspect: &'a Value, name: &str) -> &'a Value {
    inspect["params"].as_array().and_then(|a| a.iter().find(|p| p["name"] == name)).unwrap_or(&Value::Null)
}

fn listed<'a>(list: &'a Value, name: &str) -> &'a Value {
    list["parameters"].as_array().and_then(|a| a.iter().find(|p| p["name"] == name)).unwrap_or(&Value::Null)
}

/// #10747: `document.inspect` reports a parameter's `value` in millimetres (degrees for an
/// angle) like every other reply, with the parameter's own unit in `display_value`.
#[test]
fn inspect_reports_the_same_unit_as_every_other_reply() {
    let mut s = Session::default();
    run(&mut s, "document.units", json!({"units": "in"}));
    run(&mut s, "parameters.add", json!({"name": "L", "expression": "2"}));
    run(&mut s, "parameters.change", json!({"name": "L", "expression": "3"}));
    run(&mut s, "parameters.add", json!({"name": "ang", "expression": "30 deg"}));
    let i = run(&mut s, "document.inspect", json!({}));
    // A length is millimetres, whatever the parameter's own unit, and free of float noise.
    assert_eq!(param(&i, "L")["value"], 76.2, "{i}");
    // The parameter's own unit is in `display_value`, and `unit` still names it.
    assert_eq!(param(&i, "L")["display_value"], 3.0, "{i}");
    assert_eq!(param(&i, "L")["unit"], "in", "{i}");
    // An angle is degrees, like every other reply, and without the float noise.
    assert_eq!(param(&i, "ang")["value"], 30.0, "{i}");
    assert_eq!(param(&i, "ang")["display_value"], 30.0, "{i}");
    // The same numbers as `parameters.list` reports: one meaning of `value` in every reply.
    let l = run(&mut s, "parameters.list", json!({}));
    for name in ["L", "ang"] {
        assert_eq!(param(&i, name)["value"], listed(&l, name)["value"], "{name}: {i} {l}");
        assert_eq!(param(&i, name)["display_value"], listed(&l, name)["display_value"], "{name}: {i} {l}");
    }
}

/// #10747: `display_value` is the number in the parameter's OWN unit, whatever the design's
/// units are and whatever the kind of the value is: a length in centimetres in an inch design
/// is millimetres in `value` and centimetres in `display_value`, an angle in radians is degrees
/// in `value` and radians in `display_value`, and a unit-less number is the same in both.
#[test]
fn inspect_reports_the_display_value_in_the_parameters_own_unit() {
    let mut s = Session::default();
    run(&mut s, "document.units", json!({"units": "in"}));
    run(&mut s, "parameters.add", json!({"name": "w", "expression": "2 cm", "unit": "cm"}));
    run(&mut s, "parameters.add", json!({"name": "r", "expression": "0.5 rad", "unit": "rad"}));
    run(&mut s, "parameters.add", json!({"name": "u", "expression": "3", "unit": ""}));
    let i = run(&mut s, "document.inspect", json!({}));
    // A length in centimetres: millimetres in `value`, centimetres in `display_value`.
    assert_eq!(param(&i, "w")["value"], 20.0, "{i}");
    assert_eq!(param(&i, "w")["display_value"], 2.0, "{i}");
    assert_eq!(param(&i, "w")["unit"], "cm", "{i}");
    // An angle in radians: degrees in `value` (like every other reply), radians in `display_value`.
    assert_eq!(param(&i, "r")["value"], 28.6478897565, "{i}");
    assert_eq!(param(&i, "r")["display_value"], 0.5, "{i}");
    assert_eq!(param(&i, "r")["unit"], "rad", "{i}");
    // A unit-less number: the same number in both.
    assert_eq!(param(&i, "u")["value"], 3.0, "{i}");
    assert_eq!(param(&i, "u")["display_value"], 3.0, "{i}");
    // The same numbers as `parameters.list` reports, for every parameter of the design.
    let l = run(&mut s, "parameters.list", json!({}));
    for name in ["w", "r", "u"] {
        assert_eq!(param(&i, name)["value"], listed(&l, name)["value"], "{name}: {i} {l}");
        assert_eq!(param(&i, name)["display_value"], listed(&l, name)["display_value"], "{name}: {i} {l}");
    }
}
