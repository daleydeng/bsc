use bluesim::{Engine, Model, SIMIR_SCHEMA_VERSION};

const TINY: &str = include_str!("fixtures/tiny.bsim.json");
const LOCALS: &str = include_str!("fixtures/locals.bsim.json");
const UPSTREAM_STEP_GOLDEN: &str =
    include_str!("../../../testsuite/bsc.bluesim/interactive/mkTest_step.out.expected");

#[test]
fn tiny_step_matches_the_legacy_interactive_golden() {
    let model = Model::from_json(TINY).expect("valid M0 SimIR fixture");
    let mut engine = Engine::new(model).unwrap();
    let result = engine.step(10).unwrap();

    assert_eq!(result.exit_status, None);
    assert_eq!(result.time, 100);
    assert_eq!(result.output.join("\n") + "\n", UPSTREAM_STEP_GOLDEN);
}

#[test]
fn tiny_runs_to_its_legacy_finish_status() {
    let model = Model::from_json(TINY).unwrap();
    let mut engine = Engine::new(model).unwrap();
    let result = engine.run(101).unwrap();

    assert_eq!(result.exit_status, Some(0));
    assert_eq!(result.cycles, 101);
    assert_eq!(result.time, 1010);
    assert_eq!(result.output.len(), 100);
    assert_eq!(
        result.output.first().map(String::as_str),
        Some("                  10:     0")
    );
    assert_eq!(
        result.output.last().map(String::as_str),
        Some("                1000:    99")
    );
}

#[test]
fn local_values_and_time_use_the_current_cycle_snapshot() {
    let model = Model::from_json(LOCALS).unwrap();
    let mut engine = Engine::new(model).unwrap();
    let result = engine.step(1).unwrap();

    assert_eq!(result.output, [" 7@10"]);
    assert_eq!(result.exit_status, Some(3));
    assert_eq!(result.time, 10);
}

#[test]
fn rejects_unknown_schema_versions() {
    let source = TINY.replacen(
        &format!("\"schemaVersion\": {SIMIR_SCHEMA_VERSION}"),
        "\"schemaVersion\": 999",
        1,
    );
    let error = Model::from_json(&source).unwrap_err();
    assert!(error.to_string().contains("unsupported schema version 999"));
}

#[test]
fn rejects_unknown_state_references() {
    let mut source: serde_json::Value = serde_json::from_str(TINY).unwrap();
    *source
        .pointer_mut("/schedules/0/actions/0/condition/args/0/id")
        .unwrap() = serde_json::Value::String("missing".to_owned());

    let error = Model::from_json(&source.to_string()).unwrap_err();
    assert!(error.to_string().contains("unknown state \"missing\""));
}
