use bluesim::{Engine, Model};
use serde_json::{Number, Value};

const WIDE_V4: &str = include_str!("fixtures/wide-v4.bsim.json");

fn fixture() -> Value {
    serde_json::from_str(WIDE_V4).expect("fixture JSON")
}

fn assert_validation_error(mut model: Value, pointer: &str, replacement: Value, expected: &str) {
    *model.pointer_mut(pointer).expect("valid fixture pointer") = replacement;
    let error = Model::from_json(&model.to_string()).expect_err("mutated fixture must be rejected");
    assert!(
        error.to_string().contains(expected),
        "unexpected error: {error}"
    );
}

#[test]
fn v4_runs_a_wide_product_and_formats_twos_complement_decimal() {
    let model = Model::from_json(WIDE_V4).expect("valid v4 SimIR fixture");
    let mut engine = Engine::new(model).unwrap();
    let result = engine.run(1).expect("fixture finishes in one cycle");

    assert_eq!(result.exit_status, Some(0));
    assert_eq!(result.cycles, 1);
    assert_eq!(result.time, 10);
    assert_eq!(
        result.output,
        ["unsigned=73786967498736795649, signed=-8796101410815"]
    );
}

#[test]
fn v4_still_accepts_legacy_numeric_bit_values() {
    let mut model = fixture();
    *model.pointer_mut("/state/1/initialValue").unwrap() = Value::Number(Number::from(7));
    *model
        .pointer_mut("/schedules/0/actions/4/value/args/1/value")
        .unwrap() = Value::Number(Number::from(0));

    Model::from_json(&model.to_string()).expect("v4 accepts legacy numeric values");
}

#[test]
fn v4_rejects_noncanonical_or_out_of_range_bit_values() {
    assert_validation_error(
        fixture(),
        "/state/2/initialValue",
        Value::String("00".to_owned()),
        "canonical unsigned decimal string",
    );
    assert_validation_error(
        fixture(),
        "/state/2/initialValue",
        Value::String("73786976294838206464".to_owned()),
        "does not fit its 66-bit width",
    );
}

#[test]
fn v4_rejects_invalid_extract_mux_and_multiply_shapes() {
    assert_validation_error(
        fixture(),
        "/schedules/0/actions/0/value/lsb",
        Value::Number(Number::from(43)),
        "extract range must fit",
    );
    assert_validation_error(
        fixture(),
        "/schedules/0/actions/3/value/then/id",
        Value::String("missing".to_owned()),
        "unknown local \"missing\"",
    );
    assert_validation_error(
        fixture(),
        "/schedules/0/actions/5/value/width",
        Value::Number(Number::from(65)),
        "multiply result width must equal the operand width sum",
    );
}
