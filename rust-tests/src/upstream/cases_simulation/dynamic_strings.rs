//! Origin: `testsuite/bsc.evaluator/dynamic/strings/dynamic_strings.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    VcdExpectation,
};

const FIXTURE_DIR: &str = "testsuite/bsc.evaluator/dynamic/strings";

macro_rules! string_scenario {
    ($constant:ident, $module:literal) => {
        string_scenario!($constant, $module, Requirement::VerilogEnabled);
    };
    ($constant:ident, $module:literal, $icarus_requirement:expr) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.evaluator/dynamic/strings::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[
                concat!($module, ".bsv"),
                concat!("sys", $module, ".out.expected"),
            ],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeout: $crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.evaluator/dynamic/strings::", $module, "::bluesim"),
                    expected: concat!("sys", $module, ".out.expected"),
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Bluesim,
                    vcd: VcdExpectation::BluesimOutputMatchesNormal,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.evaluator/dynamic/strings::", $module, "::icarus"),
                    expected: concat!("sys", $module, ".out.expected"),
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Icarus,
                    vcd: VcdExpectation::IcarusSmoke,
                    requirement: $icarus_requirement,
                },
            ],
        };
    };
}

string_scenario!(MUX, "StringMux");
string_scenario!(CONCAT, "StringConcat");
string_scenario!(INTEGER, "StringInteger", Requirement::IcarusAtLeast(12));
string_scenario!(
    INTEGER_WITH_NULL,
    "StringIntegerWithNull",
    Requirement::IcarusAtLeast(13)
);
string_scenario!(EQ, "StringEQ");
string_scenario!(LT, "StringLT");
string_scenario!(FORMAT, "DynamicFormatString");

pub(super) const SCENARIOS: &[SimulationScenario] =
    &[MUX, CONCAT, INTEGER, INTEGER_WITH_NULL, EQ, LT, FORMAT];
