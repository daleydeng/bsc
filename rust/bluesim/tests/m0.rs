use bluesim::{Engine, Model, SIMIR_SCHEMA_V1};

const TINY: &str = include_str!("fixtures/tiny.bsim.json");
const LOCALS: &str = include_str!("fixtures/locals.bsim.json");
const UNARY_NOT: &str = r#"
{
  "schemaVersion": 1,
  "producer": { "name": "bluesim-test", "version": "m0" },
  "top": "unary_not",
  "clocks": [{ "id": "CLK", "period": 10, "activeEdge": "posedge" }],
  "state": [{ "id": "guard", "width": 1, "initialValue": 0 }],
  "schedules": [{
    "clock": "CLK",
    "actions": [{
      "kind": "if",
      "condition": {
        "kind": "unary",
        "width": 1,
        "op": "not",
        "arg": { "kind": "state", "id": "guard" }
      },
      "then": [{ "kind": "finish", "status": 0 }],
      "else": []
    }]
  }]
}
"#;
const UPSTREAM_STEP_GOLDEN: &str =
    include_str!("../../../testsuite/bsc.bluesim/interactive/mkTest_step.out.expected");
const MCD_M2: &str = r#"
{
  "schemaVersion": 2,
  "producer": { "name": "bluesim-test", "version": "m2" },
  "top": "mkMCDTest",
  "clocks": [
    {
      "id": "CLK", "period": 10, "activeEdge": "posedge", "order": 0,
      "initialValue": "low", "firstEdge": 0, "highDuration": 5, "lowDuration": 5
    },
    {
      "id": "clk2$CLK_OUT", "period": 7, "activeEdge": "posedge", "order": 1,
      "initialValue": "low", "firstEdge": 2, "highDuration": 3, "lowDuration": 4
    }
  ],
  "state": [
    { "id": "flip", "width": 1, "initialValue": 0 },
    { "id": "count", "width": 8, "initialValue": 0 },
    { "id": "rst2$OUT_RST", "width": 1, "initialValue": 0 }
  ],
  "resets": [{
    "id": "rst2", "signal": "rst2$OUT_RST", "clock": "clk2$CLK_OUT", "cycles": 2,
    "targets": [{ "state": "count", "value": 0 }]
  }],
  "schedules": [
    {
      "clock": "CLK",
      "actions": [{
        "kind": "write", "state": "flip",
        "value": {
          "kind": "unary", "width": 1, "op": "not",
          "arg": { "kind": "state", "id": "flip" }
        }
      }]
    },
    {
      "clock": "clk2$CLK_OUT",
      "actions": [
        {
          "kind": "if",
          "condition": {
            "kind": "binary", "width": 1, "op": "unsigned_less_than",
            "args": [
              { "kind": "state", "id": "count" },
              { "kind": "const", "width": 8, "value": 21 }
            ]
          },
          "then": [{
            "kind": "write", "state": "count",
            "value": {
              "kind": "binary", "width": 8, "op": "add",
              "args": [
                { "kind": "state", "id": "count" },
                { "kind": "const", "width": 8, "value": 1 }
              ]
            }
          }],
          "else": [{
            "kind": "if",
            "condition": {
              "kind": "unary", "width": 1, "op": "not",
              "arg": {
                "kind": "binary", "width": 1, "op": "equal",
                "args": [
                  { "kind": "state", "id": "rst2$OUT_RST" },
                  { "kind": "const", "width": 1, "value": 0 }
                ]
              }
            },
            "then": [{ "kind": "finish", "status": 0 }],
            "else": []
          }]
        },
        { "kind": "reset_tick", "reset": "rst2" }
      ]
    }
  ]
}
"#;
const HIERARCHY_M3: &str = r#"
{
  "schemaVersion": 3,
  "producer": { "name": "bluesim-test", "version": "m3" },
  "top": "mkHierarchy",
  "clocks": [{ "id": "CLK", "period": 10, "activeEdge": "posedge" }],
  "state": [
    { "id": "worker.x", "width": 8, "initialValue": 1 },
    { "id": "worker.y", "width": 8, "initialValue": 3 }
  ],
  "schedules": [{
    "clock": "CLK",
    "actions": [{
      "kind": "if",
      "condition": {
        "kind": "binary", "width": 1, "op": "and",
        "args": [
          {
            "kind": "unary", "width": 1, "op": "not",
            "arg": {
              "kind": "binary", "width": 1, "op": "equal",
              "args": [
                { "kind": "state", "id": "worker.y" },
                { "kind": "const", "width": 8, "value": 0 }
              ]
            }
          },
          {
            "kind": "binary", "width": 1, "op": "equal",
            "args": [
              { "kind": "state", "id": "worker.x" },
              { "kind": "const", "width": 8, "value": 1 }
            ]
          }
        ]
      },
      "then": [{
        "kind": "write", "state": "worker.y",
        "value": {
          "kind": "binary", "width": 8, "op": "sub",
          "args": [
            { "kind": "state", "id": "worker.y" },
            { "kind": "state", "id": "worker.x" }
          ]
        }
      }],
      "else": [{ "kind": "finish", "status": 0 }]
    }]
  }]
}
"#;

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
fn unary_not_controls_a_guard() {
    let model = Model::from_json(UNARY_NOT).unwrap();
    let mut engine = Engine::new(model).unwrap();
    let result = engine.step(1).unwrap();

    assert_eq!(result.exit_status, Some(0));
}

#[test]
fn m2_clockgen_and_initial_reset_finish_at_the_legacy_time() {
    let model = Model::from_json(MCD_M2).expect("valid M2 SimIR fixture");
    let mut engine = Engine::new(model).unwrap();
    let result = engine.run(100).expect("MCD fixture should finish");

    assert_eq!(result.exit_status, Some(0));
    assert_eq!(result.time, 163);
    assert_eq!(result.cycles, 41);
    assert!(result.output.is_empty());
}

#[test]
fn m3_flattened_hierarchy_runs_with_bitwise_and_subtraction() {
    let model = Model::from_json(HIERARCHY_M3).expect("valid M3 SimIR fixture");
    let mut engine = Engine::new(model).unwrap();
    let result = engine.run(10).expect("M3 hierarchy fixture should finish");

    assert_eq!(result.exit_status, Some(0));
    assert_eq!(result.time, 40);
    assert_eq!(result.cycles, 4);
    assert!(result.output.is_empty());
}

#[test]
fn rejects_unknown_schema_versions() {
    let source = TINY.replacen(
        &format!("\"schemaVersion\": {SIMIR_SCHEMA_V1}"),
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
