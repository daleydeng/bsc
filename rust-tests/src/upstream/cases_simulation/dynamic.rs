//! Origin: `testsuite/bsc.evaluator/dynamic/dynamic.exp`.

use super::SimulationCase;

const FIXTURE_DIR: &str = "testsuite/bsc.evaluator/dynamic";

macro_rules! dynamic_cases {
    ($bluesim:ident, $icarus:ident, $module:literal) => {
        pub(super) const $bluesim: SimulationCase = bluesim_case!(
            concat!("bsc.evaluator/dynamic::", $module, "::bluesim"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
        pub(super) const $icarus: SimulationCase = icarus_case!(
            concat!("bsc.evaluator/dynamic::", $module, "::icarus"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
    };
}

dynamic_cases!(INTEGER_BLUESIM, INTEGER_ICARUS, "DynamicInteger");
dynamic_cases!(
    INTEGER_NESTED_BLUESIM,
    INTEGER_NESTED_ICARUS,
    "DynamicIntegerNested"
);
dynamic_cases!(DIV_BLUESIM, DIV_ICARUS, "DynamicDiv");
dynamic_cases!(NEG_BLUESIM, NEG_ICARUS, "DynamicNeg");
dynamic_cases!(NEG_2_BLUESIM, NEG_2_ICARUS, "DynamicNeg2");
dynamic_cases!(LT_BLUESIM, LT_ICARUS, "DynamicLT");
dynamic_cases!(ADD_BLUESIM, ADD_ICARUS, "DynamicAdd");

pub(super) const CASES: &[SimulationCase] = &[
    INTEGER_BLUESIM,
    INTEGER_ICARUS,
    INTEGER_NESTED_BLUESIM,
    INTEGER_NESTED_ICARUS,
    DIV_BLUESIM,
    DIV_ICARUS,
    NEG_BLUESIM,
    NEG_ICARUS,
    NEG_2_BLUESIM,
    NEG_2_ICARUS,
    LT_BLUESIM,
    LT_ICARUS,
    ADD_BLUESIM,
    ADD_ICARUS,
];
