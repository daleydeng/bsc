use bluesim::{Engine, Model, SIMIR_SCHEMA_V2, SIMIR_SCHEMA_V5};
use serde_json::{json, Value};

const WIRE: &str = include_str!("fixtures/wire.bsim.json");
const TINY: &str = include_str!("fixtures/tiny.bsim.json");

fn fixture() -> Value {
    serde_json::from_str(WIRE).expect("fixture JSON")
}

#[test]
fn same_edge_wire_set_then_read_sees_the_new_value_and_validity() {
    let model = Model::from_json(WIRE).expect("valid Wire fixture");
    let mut engine = Engine::new(model).expect("engine");

    let result = engine.step(1).expect("first edge");

    assert_eq!(result.output, ["set=7 valid=1"]);
}

#[test]
fn wire_tick_clears_validity_while_retaining_the_value() {
    let model = Model::from_json(WIRE).expect("valid Wire fixture");
    let mut engine = Engine::new(model).expect("engine");

    engine.step(1).expect("set edge");
    let result = engine.step(1).expect("following edge");

    assert_eq!(result.output, ["next=7 valid=0"]);
}

#[test]
fn following_edge_sees_an_invalid_wire_with_its_last_value() {
    let model = Model::from_json(WIRE).expect("valid Wire fixture");
    let mut engine = Engine::new(model).expect("engine");

    let result = engine.step(2).expect("two edges");

    assert_eq!(result.output, ["set=7 valid=1", "next=7 valid=0"]);
}

#[test]
fn wire_schema_rejects_unknown_duplicate_and_width_mismatched_wires() {
    let mut unknown = fixture();
    *unknown
        .pointer_mut("/schedules/0/actions/0/else/0/wire")
        .expect("wire set") = Value::String("missing".to_owned());
    assert!(Model::from_json(&unknown.to_string())
        .expect_err("unknown wire")
        .to_string()
        .contains("unknown wire \"missing\""));

    let mut mismatch = fixture();
    *mismatch
        .pointer_mut("/schedules/0/actions/0/else/0/value/width")
        .expect("wire set value") = json!(7);
    assert!(Model::from_json(&mismatch.to_string())
        .expect_err("wire width mismatch")
        .to_string()
        .contains("has 7-bit value; expected 8-bit value"));

    let mut duplicate = fixture();
    let wire = duplicate["primitives"][0].clone();
    duplicate["primitives"].as_array_mut().unwrap().push(wire);
    assert!(Model::from_json(&duplicate.to_string())
        .expect_err("duplicate wire")
        .to_string()
        .contains("duplicate wire id \"sample\""));
}

#[test]
fn wire_schema_requires_one_final_wire_tick() {
    let mut missing_tick = fixture();
    missing_tick["schedules"][0]["actions"]
        .as_array_mut()
        .unwrap()
        .pop();

    assert!(Model::from_json(&missing_tick.to_string())
        .expect_err("missing final tick")
        .to_string()
        .contains("exactly one final wire tick"));
}

#[test]
fn pre_wire_schema_versions_reject_wire_declarations_actions_and_expressions() {
    for version in 1..SIMIR_SCHEMA_V5 {
        let mut declaration = valid_model_for(version);
        declaration["primitives"] = json!([{
            "kind": "wire",
            "id": "sample",
            "width": 8,
            "initialValue": 0,
            "initialValid": false
        }]);
        assert_rejected(
            &declaration,
            "primitive declarations require schema version 5 or later",
        );

        let mut action = valid_model_for(version);
        action["schedules"][0]["actions"] = json!([{ "kind": "wire_tick" }]);
        assert_rejected(&action, "wire tick requires schema version 5 or later");

        let mut expression = valid_model_for(version);
        expression["schedules"][0]["actions"] = json!([
            {
                "kind": "let",
                "local": "sample",
                "value": { "kind": "wire_value", "wire": "sample" }
            },
            { "kind": "finish", "status": 0 }
        ]);
        assert_rejected(
            &expression,
            "wire value expression requires schema version 5 or later",
        );
    }
}

fn valid_model_for(version: u32) -> Value {
    let mut model: Value = serde_json::from_str(TINY).expect("tiny fixture JSON");
    model["schemaVersion"] = json!(version);
    if version == SIMIR_SCHEMA_V2 {
        let clock = &mut model["clocks"][0];
        clock["order"] = json!(0);
        clock["initialValue"] = json!("low");
        clock["firstEdge"] = json!(0);
        clock["highDuration"] = json!(5);
        clock["lowDuration"] = json!(5);
    }
    model
}

fn assert_rejected(model: &Value, expected: &str) {
    let error =
        Model::from_json(&model.to_string()).expect_err("unsupported Wire form must be rejected");
    assert!(
        error.to_string().contains(expected),
        "expected {expected:?}, got {error}"
    );
}
